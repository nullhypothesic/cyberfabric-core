use credstore_sdk::error::CredStoreError;
use thiserror::Error;

use crate::infra::crypto::CryptoError;
use crate::infra::db::repo::error::RepositoryError;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("validation error on field '{0}': {1}")]
    Validation(String, String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("foreign key violation")]
    ForeignKeyViolation,

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("internal: {0}")]
    Internal(String),
}

impl From<RepositoryError> for ServiceError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::NotFound => ServiceError::NotFound("record".to_string()),
            RepositoryError::AlreadyExists(msg) => ServiceError::AlreadyExists(msg),
            RepositoryError::Forbidden => ServiceError::Forbidden("access denied".to_string()),
            RepositoryError::ForeignKeyViolation => ServiceError::ForeignKeyViolation,
            RepositoryError::Database(e) => ServiceError::Database(e.to_string()),
            RepositoryError::Scope(msg) => ServiceError::Forbidden(msg),
        }
    }
}

impl From<CryptoError> for ServiceError {
    fn from(e: CryptoError) -> Self {
        ServiceError::Crypto(e.to_string())
    }
}

impl From<ServiceError> for CredStoreError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::NotFound(_) => CredStoreError::NotFound,
            ServiceError::Database(msg) => CredStoreError::ServiceUnavailable(msg),
            _ => CredStoreError::Internal(e.to_string()),
        }
    }
}
