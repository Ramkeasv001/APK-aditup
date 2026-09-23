use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::error::SecurityError;

struct LimiterState {
    failures: u32,
    locked_until: Option<Instant>,
}

/// Generic failed-attempt lockout, shared by login and Admin PIN elevation
/// (§4/§9 of the architecture doc both call for a cooldown after repeated
/// failures). `now` is passed in by the caller rather than read from the
/// system clock internally, so this is deterministically unit-testable
/// without real sleeps and without any async/mocking machinery.
pub struct AttemptLimiter {
    max_attempts: u32,
    lockout: Duration,
    state: Mutex<LimiterState>,
}

impl AttemptLimiter {
    pub fn new(max_attempts: u32, lockout: Duration) -> Self {
        Self {
            max_attempts,
            lockout,
            state: Mutex::new(LimiterState {
                failures: 0,
                locked_until: None,
            }),
        }
    }

    /// Call before attempting a credential check. Errors with the remaining
    /// cooldown if locked out; otherwise clears an expired lockout so the
    /// caller gets a fresh set of attempts.
    pub fn check(&self, now: Instant) -> Result<(), SecurityError> {
        let mut state = self.state.lock().unwrap();
        if let Some(until) = state.locked_until {
            if now < until {
                return Err(SecurityError::LockedOut {
                    retry_after_secs: (until - now).as_secs().max(1),
                });
            }
            state.failures = 0;
            state.locked_until = None;
        }
        Ok(())
    }

    pub fn record_failure(&self, now: Instant) {
        let mut state = self.state.lock().unwrap();
        state.failures += 1;
        if state.failures >= self.max_attempts {
            state.locked_until = Some(now + self.lockout);
        }
    }

    pub fn record_success(&self) {
        let mut state = self.state.lock().unwrap();
        state.failures = 0;
        state.locked_until = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_attempts_under_the_limit() {
        let limiter = AttemptLimiter::new(3, Duration::from_secs(60));
        let t0 = Instant::now();

        limiter.check(t0).unwrap();
        limiter.record_failure(t0);
        limiter.check(t0).unwrap();
        limiter.record_failure(t0);
        limiter.check(t0).unwrap(); // 2 failures so far, limit is 3
    }

    #[test]
    fn locks_out_once_max_attempts_is_reached() {
        let limiter = AttemptLimiter::new(3, Duration::from_secs(60));
        let t0 = Instant::now();

        limiter.record_failure(t0);
        limiter.record_failure(t0);
        limiter.record_failure(t0); // 3rd failure trips the lockout

        let result = limiter.check(t0 + Duration::from_secs(1));
        assert!(matches!(result, Err(SecurityError::LockedOut { .. })));
    }

    #[test]
    fn lockout_expires_after_the_configured_duration() {
        let limiter = AttemptLimiter::new(2, Duration::from_secs(30));
        let t0 = Instant::now();

        limiter.record_failure(t0);
        limiter.record_failure(t0);
        assert!(limiter.check(t0 + Duration::from_secs(10)).is_err());

        // Past the 30s cooldown, a fresh set of attempts is granted.
        assert!(limiter.check(t0 + Duration::from_secs(31)).is_ok());
        limiter.record_failure(t0 + Duration::from_secs(31));
        assert!(limiter.check(t0 + Duration::from_secs(32)).is_ok());
    }

    #[test]
    fn record_success_resets_the_failure_count() {
        let limiter = AttemptLimiter::new(2, Duration::from_secs(60));
        let t0 = Instant::now();

        limiter.record_failure(t0);
        limiter.record_success();
        limiter.record_failure(t0);
        // Only 1 failure since the reset — still under the limit of 2.
        assert!(limiter.check(t0).is_ok());
    }
}
