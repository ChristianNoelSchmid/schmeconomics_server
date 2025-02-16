use axum::response::IntoResponse;
use log::error;
use thiserror::Error;

use crate::response::internal_server_error_response;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Reqwest error while attempting to fetch conversion: {0}")]
    ClientFetchError(#[from] reqwest::Error),
    #[error("Status code other than 200 received from API. StatusCode: {0}. Body: {1}")]
    StatusCodeFetchError(reqwest::StatusCode, String),
    #[error("Could not parse response body after fetching conversion. Error: {0}")]
    ParseError(#[from] serde_json::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        error!("{}", self);
        internal_server_error_response()
    }
}