use modkit_db::secure::ScopeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("record not found")]
    NotFound,

    #[error("record already exists: {0}")]
    AlreadyExists(String),

    #[error("forbidden")]
    Forbidden,

    #[error("foreign key violation")]
    ForeignKeyViolation,

    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    /// Access denied or tenant boundary violation from SecureEntityExt.
    #[error("scope error: {0}")]
    Scope(String),
}

impl From<ScopeError> for RepositoryError {
    fn from(e: ScopeError) -> Self {
        match e {
            ScopeError::Db(db_err) => RepositoryError::Database(db_err),
            other => RepositoryError::Scope(other.to_string()),
        }
    }
}
