use axum::http::StatusCode;
use modkit::api::problem::Problem;

use crate::domain::error::ServiceError;

impl From<ServiceError> for Problem {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::NotFound(msg) => Problem::new(
                StatusCode::NOT_FOUND,
                "Not Found",
                format!("Resource not found: {msg}"),
            ),
            ServiceError::Validation(field, msg) => Problem::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Validation Error",
                format!("Validation error on '{field}': {msg}"),
            ),
            ServiceError::Forbidden(msg) => Problem::new(
                StatusCode::FORBIDDEN,
                "Forbidden",
                format!("Access denied: {msg}"),
            ),
            ServiceError::AlreadyExists(msg) => Problem::new(
                StatusCode::CONFLICT,
                "Conflict",
                format!("Already exists: {msg}"),
            ),
            ServiceError::ForeignKeyViolation => Problem::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Foreign Key Violation",
                "Referenced resource does not exist".to_string(),
            ),
            ServiceError::Crypto(msg) => {
                tracing::error!("Crypto error: {}", msg);
                Problem::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    "An internal error occurred".to_string(),
                )
            }
            ServiceError::Database(msg) => {
                tracing::error!("Database error: {}", msg);
                Problem::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    "A database error occurred".to_string(),
                )
            }
            ServiceError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                Problem::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    "An internal error occurred".to_string(),
                )
            }
        }
    }
}
