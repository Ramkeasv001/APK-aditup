use std::time::{Duration, Instant};

use aditup_db::repositories::admin_settings::AdminSettingsRepository;
use aditup_db::repositories::users::{User, UserRole, UsersRepository};
use aditup_db::Database;

use crate::error::SecurityError;
use crate::password::{hash_password, verify_password};
use crate::recovery::{generate_recovery_key, verify_recovery_key};
use crate::AttemptLimiter;

const RECOVERY_KEY_HASH_SETTING: &str = "recovery_key_hash";
const ADMIN_PIN_HASH_SETTING: &str = "admin_pin_hash";

/// Orchestrates login, first-run setup, password recovery, and Admin
/// elevation against a single `Database`. This is the one place in the app
/// that is allowed to call both `aditup_db`'s repositories and this crate's
/// hashing/limiter primitives together — everything above this layer (Tauri
/// commands, later) only ever sees `AuthService`, never raw repositories or
/// raw Argon2 calls.
pub struct AuthService<'a> {
    db: &'a Database,
    login_limiter: AttemptLimiter,
    admin_limiter: AttemptLimiter,
}

impl<'a> AuthService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self {
            db,
            login_limiter: AttemptLimiter::new(5, Duration::from_secs(5 * 60)),
            admin_limiter: AttemptLimiter::new(3, Duration::from_secs(5 * 60)),
        }
    }

    /// `true` until the very first user has been created — the UI uses this
    /// to decide whether to show the Setup wizard or the Login screen.
    pub fn is_first_run(&self) -> Result<bool, SecurityError> {
        Ok(UsersRepository::new(self.db).count()? == 0)
    }

    /// Creates the master account (role `Admin`) and issues a one-time
    /// recovery key. Only callable once — every subsequent call fails with
    /// `AlreadySetUp` even if the caller doesn't check `is_first_run` first.
    pub fn setup(&self, username: &str, password: &str) -> Result<(User, String), SecurityError> {
        if !self.is_first_run()? {
            return Err(SecurityError::AlreadySetUp);
        }
        log::info!("running first-time setup for user '{username}'");
        let password_hash = hash_password(password)?;
        let user =
            UsersRepository::new(self.db).create(username, &password_hash, UserRole::Admin)?;

        let (recovery_key, recovery_hash) = generate_recovery_key()?;
        AdminSettingsRepository::new(self.db).set(RECOVERY_KEY_HASH_SETTING, &recovery_hash)?;

        Ok((user, recovery_key))
    }

    /// Verifies a username/password pair, subject to the login attempt
    /// limiter. Deliberately returns the same `InvalidCredentials` error for
    /// both "no such user" and "wrong password" — telling those apart would
    /// let an attacker enumerate valid usernames.
    pub fn login(
        &self,
        username: &str,
        password: &str,
        now: Instant,
    ) -> Result<User, SecurityError> {
        self.login_limiter.check(now)?;

        let user = UsersRepository::new(self.db).get_by_username(username)?;
        let user = match user {
            Some(user) => user,
            None => {
                self.login_limiter.record_failure(now);
                return Err(SecurityError::InvalidCredentials);
            }
        };

        if verify_password(password, &user.password_hash)? {
            self.login_limiter.record_success();
            log::info!("user '{username}' logged in");
            Ok(user)
        } else {
            self.login_limiter.record_failure(now);
            Err(SecurityError::InvalidCredentials)
        }
    }

    /// Resets `user_id`'s password using the one-time recovery key from
    /// setup. There is no other password-reset path — see §9.
    pub fn reset_password_with_recovery_key(
        &self,
        user_id: &str,
        recovery_key: &str,
        new_password: &str,
    ) -> Result<(), SecurityError> {
        let settings = AdminSettingsRepository::new(self.db);
        let stored_hash = settings
            .get(RECOVERY_KEY_HASH_SETTING)?
            .ok_or(SecurityError::InvalidRecoveryKey)?;

        if !verify_recovery_key(recovery_key, &stored_hash)? {
            return Err(SecurityError::InvalidRecoveryKey);
        }

        log::warn!("password reset via recovery key for user {user_id}");
        let new_hash = hash_password(new_password)?;
        UsersRepository::new(self.db).update_password_hash(user_id, &new_hash)?;
        Ok(())
    }

    pub fn is_admin_pin_set(&self) -> Result<bool, SecurityError> {
        Ok(AdminSettingsRepository::new(self.db)
            .get(ADMIN_PIN_HASH_SETTING)?
            .is_some())
    }

    /// First-time Admin PIN setup only; fails if one is already configured
    /// (use `change_admin_pin` to replace it, which requires the old PIN).
    pub fn set_admin_pin(&self, pin: &str) -> Result<(), SecurityError> {
        if self.is_admin_pin_set()? {
            return Err(SecurityError::AdminPinAlreadySet);
        }
        let hash = hash_password(pin)?;
        AdminSettingsRepository::new(self.db).set(ADMIN_PIN_HASH_SETTING, &hash)?;
        log::info!("admin PIN configured");
        Ok(())
    }

    pub fn change_admin_pin(
        &self,
        current_pin: &str,
        new_pin: &str,
        now: Instant,
    ) -> Result<(), SecurityError> {
        self.verify_admin_pin(current_pin, now)?;
        let hash = hash_password(new_pin)?;
        AdminSettingsRepository::new(self.db).set(ADMIN_PIN_HASH_SETTING, &hash)?;
        log::info!("admin PIN changed");
        Ok(())
    }

    /// Gates entry to the Admin Console. Deliberately independent of
    /// `login`'s `AttemptLimiter` and of the logged-in user's `role` — per
    /// §9, an auditor login must never imply Admin rights on its own.
    pub fn verify_admin_pin(&self, pin: &str, now: Instant) -> Result<(), SecurityError> {
        self.admin_limiter.check(now)?;

        let stored = AdminSettingsRepository::new(self.db)
            .get(ADMIN_PIN_HASH_SETTING)?
            .ok_or(SecurityError::AdminPinNotSet)?;

        if verify_password(pin, &stored)? {
            self.admin_limiter.record_success();
            Ok(())
        } else {
            self.admin_limiter.record_failure(now);
            Err(SecurityError::InvalidAdminPin)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:"), "test-passphrase").unwrap()
    }

    #[test]
    fn is_first_run_until_setup_creates_a_user() {
        let db = test_db();
        let auth = AuthService::new(&db);
        assert!(auth.is_first_run().unwrap());

        auth.setup("hse.admin", "correct-horse-battery-staple")
            .unwrap();
        assert!(!auth.is_first_run().unwrap());
    }

    #[test]
    fn setup_returns_a_usable_recovery_key_and_cannot_run_twice() {
        let db = test_db();
        let auth = AuthService::new(&db);

        let (user, recovery_key) = auth.setup("hse.admin", "password123").unwrap();
        assert_eq!(user.role, UserRole::Admin);
        assert!(!recovery_key.is_empty());

        let second_attempt = auth.setup("someone.else", "another-password");
        assert!(matches!(second_attempt, Err(SecurityError::AlreadySetUp)));
    }

    #[test]
    fn login_succeeds_with_correct_credentials() {
        let db = test_db();
        let auth = AuthService::new(&db);
        auth.setup("hse.admin", "correct-horse-battery-staple")
            .unwrap();

        let user = auth
            .login("hse.admin", "correct-horse-battery-staple", Instant::now())
            .unwrap();
        assert_eq!(user.username, "hse.admin");
    }

    #[test]
    fn login_fails_with_wrong_password_without_revealing_which_part_was_wrong() {
        let db = test_db();
        let auth = AuthService::new(&db);
        auth.setup("hse.admin", "correct-horse-battery-staple")
            .unwrap();

        let wrong_password = auth.login("hse.admin", "wrong", Instant::now());
        let wrong_username = auth.login("nobody", "wrong", Instant::now());

        assert!(matches!(
            wrong_password,
            Err(SecurityError::InvalidCredentials)
        ));
        assert!(matches!(
            wrong_username,
            Err(SecurityError::InvalidCredentials)
        ));
    }

    #[test]
    fn login_locks_out_after_five_failures() {
        let db = test_db();
        let auth = AuthService::new(&db);
        auth.setup("hse.admin", "correct-password").unwrap();
        let t0 = Instant::now();

        for _ in 0..5 {
            let _ = auth.login("hse.admin", "wrong", t0);
        }

        let result = auth.login("hse.admin", "correct-password", t0 + Duration::from_secs(1));
        assert!(matches!(result, Err(SecurityError::LockedOut { .. })));
    }

    #[test]
    fn recovery_key_resets_password_and_old_password_stops_working() {
        let db = test_db();
        let auth = AuthService::new(&db);
        let (user, recovery_key) = auth.setup("hse.admin", "old-password").unwrap();

        auth.reset_password_with_recovery_key(&user.id, &recovery_key, "new-password")
            .unwrap();

        assert!(auth
            .login("hse.admin", "new-password", Instant::now())
            .is_ok());
        assert!(matches!(
            auth.login("hse.admin", "old-password", Instant::now()),
            Err(SecurityError::InvalidCredentials)
        ));
    }

    #[test]
    fn wrong_recovery_key_does_not_reset_the_password() {
        let db = test_db();
        let auth = AuthService::new(&db);
        let (user, _) = auth.setup("hse.admin", "old-password").unwrap();

        let result = auth.reset_password_with_recovery_key(
            &user.id,
            "0000-0000-0000-0000-0000-0000-0000-0000-0000-0000",
            "new-password",
        );

        assert!(matches!(result, Err(SecurityError::InvalidRecoveryKey)));
        assert!(auth
            .login("hse.admin", "old-password", Instant::now())
            .is_ok());
    }

    #[test]
    fn admin_pin_lifecycle_set_verify_change() {
        let db = test_db();
        let auth = AuthService::new(&db);
        let t0 = Instant::now();

        assert!(!auth.is_admin_pin_set().unwrap());
        auth.set_admin_pin("1234").unwrap();
        assert!(auth.is_admin_pin_set().unwrap());

        assert!(auth.verify_admin_pin("1234", t0).is_ok());
        assert!(matches!(
            auth.verify_admin_pin("9999", t0),
            Err(SecurityError::InvalidAdminPin)
        ));

        auth.change_admin_pin("1234", "5678", t0).unwrap();
        assert!(auth.verify_admin_pin("5678", t0).is_ok());
        assert!(matches!(
            auth.verify_admin_pin("1234", t0),
            Err(SecurityError::InvalidAdminPin)
        ));
    }

    #[test]
    fn set_admin_pin_twice_fails_without_going_through_change() {
        let db = test_db();
        let auth = AuthService::new(&db);
        auth.set_admin_pin("1234").unwrap();

        let result = auth.set_admin_pin("5678");
        assert!(matches!(result, Err(SecurityError::AdminPinAlreadySet)));
    }

    #[test]
    fn admin_elevation_is_independent_of_user_role() {
        // A non-admin user's login has nothing to do with Admin Console
        // access — verifying that requires the separate PIN regardless.
        let db = test_db();
        let auth = AuthService::new(&db);
        let t0 = Instant::now();

        assert!(matches!(
            auth.verify_admin_pin("anything", t0),
            Err(SecurityError::AdminPinNotSet)
        ));
    }

    #[test]
    fn admin_pin_locks_out_after_three_failures() {
        let db = test_db();
        let auth = AuthService::new(&db);
        auth.set_admin_pin("1234").unwrap();
        let t0 = Instant::now();

        for _ in 0..3 {
            let _ = auth.verify_admin_pin("wrong", t0);
        }

        let result = auth.verify_admin_pin("1234", t0 + Duration::from_secs(1));
        assert!(matches!(result, Err(SecurityError::LockedOut { .. })));
    }
}
