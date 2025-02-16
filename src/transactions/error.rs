use std::backtrace;

use axum::response::IntoResponse;
use log::error;
use sea_orm::{prelude::Uuid, DbErr};
use thiserror::Error;

use crate::{currency_conv_provider, db_utils::DbUtilsError, response::internal_server_error_response};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("An error occurred while invoking the CurrencyConversionProvider: {0}")]
    CurrencyConversionProviderError(#[from] currency_conv_provider::error::Error),
    #[error("An error occurred while connecting to the database: {0}")]
    DbErr(#[from] DbErr),
    #[error(transparent)]
    DbUtilsError(#[from] DbUtilsError),
    #[error("Row not found with value {0}")]
    RowNotFound(String),
    #[error("Account {0} does not own transaction {1}")]
    AccountDoesNotOwnTransaction(Uuid, i32),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        return match self {
            Error::DbErr(_) | Error::DbUtilsError(_) |
            Error::RowNotFound(_) | Error::CurrencyConversionProviderError(_) |
            Error::AccountDoesNotOwnTransaction(_, _) => {
                error!("{}\n{}", self, backtrace::Backtrace::capture());
                internal_server_error_response()
            },
        }
    }
}