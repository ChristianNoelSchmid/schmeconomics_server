use thiserror::Error;
use tokens_rs::password_hasher;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("A database error occurred: {0}")]
    DbErr(#[from] sea_orm::DbErr),
    #[error(transparent)]
    DbUtilsErr(#[from] crate::db_utils::DbUtilsError),
    #[error("An error occurred while using the password hasher: {0}")]
    PasswordHasherError(#[from] password_hasher::error::Error),
    #[error("Email {0} already in use")]
    EmailInUse(String), 
    #[error("User not found: {0}")]
    UserNotFound(Uuid),
}