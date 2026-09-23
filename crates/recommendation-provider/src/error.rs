use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error(transparent)]
    Db(#[from] aditup_db::DbError),
}
