use axum::response::IntoResponse;
use log::error;
use reqwest::StatusCode;
use thiserror::Error;
use uuid::Uuid;

use crate::{response::internal_server_error_response, validations};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("A database error occurred: {0}")]
    DbErr(#[from] sea_orm::DbErr),
    #[error(transparent)]
    DbUtilsErr(#[from] crate::db_utils::DbUtilsError),
    #[error(transparent)]
    ServiceError(Box<dyn std::error::Error + Send + Sync>),
    #[error("Account not found with ID {0}")]
    AccountNotFound(Uuid),
    #[error("User not found with ID {0}")]
    UserNotFound(Uuid),
    #[error("Account {0} User {1} relationship not found")]
    AccountUserNotFound(Uuid, Uuid),
}  

impl From<send_email_rs::error::Error> for Error {
    fn from(value: send_email_rs::error::Error) -> Self {
        Self::ServiceError(Box::new(value))
    }
}

impl From<validations::error::Error> for Error {
    fn from(value: validations::error::Error) -> Self {
        Self::ServiceError(Box::new(value))
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::AccountNotFound(_) | Self::UserNotFound(_) | 
            Self::AccountUserNotFound(_, _) => {
                (StatusCode::BAD_REQUEST, self.to_string()).into_response()
            },
            _ => {
                error!("{:?}", self);
                internal_server_error_response()
            }
        }
    }
}