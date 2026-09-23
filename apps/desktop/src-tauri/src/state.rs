use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use aditup_db::Database;
use aditup_security::{AttemptLimiter, SessionVault};

/// The User Journey (§4) describes a single master-password unlock with no
/// username field in the Setup wizard — Mode 1 v1 is single-account. Every
/// command that needs a `users` row (auditor attribution, etc.) uses this
/// fixed value under the hood; the UI never shows it.
pub const OWNER_USERNAME: &str = "owner";

/// Shared, Tauri-managed application state.
///
/// `db` is `None` until the app is unlocked (or before Setup has ever run).
/// Every command that needs data access takes the lock and returns an error
/// if it's still `None` — there is no other way for a command to reach the
/// database, so a locked/not-yet-set-up app is enforced at this one seam
/// rather than trusted to every command individually.
pub struct AppState {
    pub db: Mutex<Option<Database>>,
    pub db_path: PathBuf,
    /// Guards the encrypted-file open attempt itself (wrong password before
    /// the database — and therefore the `users` table — is even open).
    /// Distinct from `aditup_security::AuthService`'s own login limiter,
    /// which only applies once a `Database` already exists to query.
    pub unlock_limiter: AttemptLimiter,
    pub session: SessionVault,
    /// Whether the Admin Console PIN has been verified this session.
    /// `AuthService::verify_admin_pin` is constructed fresh per command
    /// call, so its *internal* attempt limiter can't persist across calls —
    /// `admin_pin_limiter` below is what actually enforces the cooldown,
    /// the same way `unlock_limiter` stands in for `AuthService::login`'s.
    pub admin_elevated: Mutex<bool>,
    pub admin_pin_limiter: AttemptLimiter,
}

impl AppState {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db: Mutex::new(None),
            db_path,
            unlock_limiter: AttemptLimiter::new(5, Duration::from_secs(5 * 60)),
            session: SessionVault::new(Duration::from_secs(15 * 60)),
            admin_elevated: Mutex::new(false),
            admin_pin_limiter: AttemptLimiter::new(3, Duration::from_secs(5 * 60)),
        }
    }

    /// The one place every data-touching command reaches the database
    /// through. Returns a user-facing error instead of the `Database` when
    /// the app is locked or hasn't been set up yet, so individual commands
    /// never need to duplicate that check themselves.
    ///
    /// This is also the one place idle-timeout auto-lock (§4/§9) is
    /// actually enforced: `SessionVault::is_locked` is a pure time
    /// computation (see `aditup_security::SessionVault`), so without a
    /// check here an idle session would stay fully queryable forever —
    /// `session_status` would *report* locked, but nothing would act on it.
    /// A call that lands exactly at the idle boundary drops the held
    /// `Database` (matching what an explicit `lock()` does) instead of
    /// silently succeeding one more time. Every other authenticated
    /// interaction, by reaching this line at all, is activity — so it
    /// resets the idle clock via `touch`.
    pub fn with_db<T>(&self, f: impl FnOnce(&Database) -> Result<T, String>) -> Result<T, String> {
        let now = Instant::now();
        if self.session.is_locked(now) {
            *self.db.lock().unwrap() = None;
            return Err("the app is locked".to_string());
        }
        self.session.touch(now);

        let guard = self.db.lock().unwrap();
        let db = guard
            .as_ref()
            .ok_or_else(|| "the app is locked".to_string())?;
        f(db)
    }

    /// Like `with_db`, but also requires the Admin PIN to have been
    /// verified this session — every Admin Console command goes through
    /// this instead of `with_db` directly.
    pub fn with_admin<T>(
        &self,
        f: impl FnOnce(&Database) -> Result<T, String>,
    ) -> Result<T, String> {
        if !*self.admin_elevated.lock().unwrap() {
            return Err("Admin access has not been unlocked".to_string());
        }
        self.with_db(f)
    }

    /// Locking the whole app also de-elevates Admin — re-entering the
    /// master password does not imply Admin access, per §9.
    pub fn lock(&self) {
        self.session.lock();
        *self.db.lock().unwrap() = None;
        *self.admin_elevated.lock().unwrap() = false;
    }
}
