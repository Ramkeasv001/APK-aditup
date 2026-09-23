use thiserror::Error;

#[derive(Debug, Error)]
pub enum KleError {
    #[error(transparent)]
    Db(#[from] aditup_db::DbError),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("pending observation {0} is not awaiting resolution")]
    NotPending(i64),

    #[error("pending observation {0} must be approved before it can be published")]
    NotApproved(i64),
}
