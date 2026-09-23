use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("invalid username or password")]
    InvalidCredentials,

    #[error("too many failed attempts — try again in {retry_after_secs}s")]
    LockedOut { retry_after_secs: u64 },

    #[error("setup has already run — an account already exists")]
    AlreadySetUp,

    #[error("admin PIN has already been configured — use change_admin_pin instead")]
    AdminPinAlreadySet,

    #[error("admin PIN has not been configured yet")]
    AdminPinNotSet,

    #[error("invalid admin PIN")]
    InvalidAdminPin,

    #[error("invalid recovery key")]
    InvalidRecoveryKey,

    #[error(transparent)]
    Db(#[from] aditup_db::DbError),

    #[error("password hashing failed: {0}")]
    Hash(String),
}
