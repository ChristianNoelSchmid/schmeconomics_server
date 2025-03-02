use thiserror::Error;

use crate::db_utils::{ValidationContext, ValidationKind};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("A database error occurred: {0}")]
    DbErr(#[from] sea_orm::DbErr),
    #[error("Validation with token {0} not found")]
    ValidationNotFound(String),
    #[error("Validation expired. Token {0}")]
    ValidationExpired(String),
    #[error("Deserialize context error. Error: {0}")]
    DeserializeContextError(#[from] serde_json::Error),
    #[error("Mismatched validation kind and context: Kind is {0:?} but context is {1:?}")]
    MismatchedValidation(ValidationKind, ValidationContext)
}