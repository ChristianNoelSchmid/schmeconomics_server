use axum::response::IntoResponse;
use reqwest::StatusCode;
use thiserror::Error;
use tokens_rs::password_hasher;
use uuid::Uuid;

use crate::response::internal_server_error_response;

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

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::EmailInUse(_) => {
                (StatusCode::BAD_REQUEST, "Could not create user").into_response()
            },
            Self::UserNotFound(_) => {
                (StatusCode::BAD_REQUEST, "Authentication failed").into_response()
            },
            _ => {
                log::error!("{:?}", self);
                internal_server_error_response() 
            }
        }
    }
}