use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Tracks whether the app is unlocked and for how much longer, per §4/§9:
/// auto-lock after idle timeout, manual lock, re-unlock requires the
/// password again. This holds no secret material itself — it's pure
/// timing/state logic. The caller is responsible for actually dropping the
/// keyed `Database` handle (or equivalent) when this reports locked, and for
/// re-deriving the key from the password on `unlock`.
pub struct SessionVault {
    idle_timeout: Duration,
    /// `None` means locked; `Some(t)` is the timestamp of the last activity.
    last_activity: Mutex<Option<Instant>>,
}

impl SessionVault {
    pub fn new(idle_timeout: Duration) -> Self {
        Self {
            idle_timeout,
            last_activity: Mutex::new(None),
        }
    }

    /// Call after a successful login/password re-entry.
    pub fn unlock(&self, now: Instant) {
        *self.last_activity.lock().unwrap() = Some(now);
    }

    /// Call on every authenticated user interaction to reset the idle clock.
    /// A no-op while locked — activity can't resurrect a locked session,
    /// only an explicit `unlock` (i.e. re-entering the password) can.
    pub fn touch(&self, now: Instant) {
        let mut guard = self.last_activity.lock().unwrap();
        if guard.is_some() {
            *guard = Some(now);
        }
    }

    /// Explicit manual lock (a "Lock now" button), independent of idle time.
    pub fn lock(&self) {
        *self.last_activity.lock().unwrap() = None;
    }

    pub fn is_locked(&self, now: Instant) -> bool {
        match *self.last_activity.lock().unwrap() {
            None => true,
            Some(last) => now.duration_since(last) >= self.idle_timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_locked() {
        let vault = SessionVault::new(Duration::from_secs(300));
        assert!(vault.is_locked(Instant::now()));
    }

    #[test]
    fn unlock_clears_the_locked_state() {
        let vault = SessionVault::new(Duration::from_secs(300));
        let t0 = Instant::now();
        vault.unlock(t0);
        assert!(!vault.is_locked(t0));
    }

    #[test]
    fn becomes_locked_after_the_idle_timeout_without_activity() {
        let vault = SessionVault::new(Duration::from_secs(300));
        let t0 = Instant::now();
        vault.unlock(t0);

        assert!(!vault.is_locked(t0 + Duration::from_secs(299)));
        assert!(vault.is_locked(t0 + Duration::from_secs(300)));
    }

    #[test]
    fn touch_resets_the_idle_clock() {
        let vault = SessionVault::new(Duration::from_secs(300));
        let t0 = Instant::now();
        vault.unlock(t0);

        vault.touch(t0 + Duration::from_secs(250));
        // Without the touch this would be locked (250 + 250 >= 300).
        assert!(!vault.is_locked(t0 + Duration::from_secs(500)));
    }

    #[test]
    fn touch_while_locked_does_not_unlock() {
        let vault = SessionVault::new(Duration::from_secs(300));
        let t0 = Instant::now();
        vault.touch(t0); // no-op, vault was never unlocked
        assert!(vault.is_locked(t0));
    }

    #[test]
    fn explicit_lock_overrides_a_fresh_session() {
        let vault = SessionVault::new(Duration::from_secs(300));
        let t0 = Instant::now();
        vault.unlock(t0);
        vault.lock();
        assert!(vault.is_locked(t0));
    }
}
