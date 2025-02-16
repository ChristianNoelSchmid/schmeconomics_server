use axum::{http::StatusCode, response::IntoResponse};
use log::error;
use thiserror::Error;
use tokens_rs::token_service;

use crate::response::internal_server_error_response;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("TokenService error while attempting to authorize JWT: {0}")]
    TokenServiceError(#[from]token_service::error::Error),
    #[error("Unauthorized. Please use the `Authorization` header with JWT bearer token format")]
    Unauthorized,
    #[error("Could not parse authorization header")]
    ParseHeaderError
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        return match self {
            Error::Unauthorized 
                => (StatusCode::BAD_REQUEST, self.to_string()).into_response(),
            Error::ParseHeaderError 
                => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
            Error::TokenServiceError(_) => {
                error!("{}", self);
                internal_server_error_response()
            }
        };
    }
}