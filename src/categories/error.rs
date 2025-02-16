use axum::{http::StatusCode, response::IntoResponse};
use log::error;
use sea_orm::DbErr;
use thiserror::Error;
use uuid::Uuid;

use crate::{db_utils::DbUtilsError, response::internal_server_error_response};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("An error occurred while connecting to the database: {0}")]
    DbErr(#[from] DbErr),
    #[error(transparent)]
    DbUtilsError(#[from] DbUtilsError),
    #[error("User does not own acount with ID: {0}")]
    UserDoesNotOwnAccount(Uuid),
    #[error("Category name '{0}' already taken in account")]
    NameReuse(String),
    #[error("Category with ID '{0}' not found")]
    CategoryNotFound(Uuid),
    #[error("Duplicate Order ID: {0}")]
    OrderDuplicateId(Uuid),
    #[error("Duplicate Order Index: {0}")]
    OrderDuplicateIndex(i32),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        return match self {
            Error::DbErr(_) | Error::DbUtilsError(_) => { 
                error!("{}", self);
                internal_server_error_response()
            },
            Error::NameReuse(_) | Error::CategoryNotFound(_) |
            Error::OrderDuplicateId(_) | Error::OrderDuplicateIndex(_) | 
            Error::UserDoesNotOwnAccount(_) => {
                (StatusCode::BAD_REQUEST, self.to_string()).into_response()
            }
        };
    }
}