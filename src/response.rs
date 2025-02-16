use axum::{http::StatusCode, response::{IntoResponse, Response}};


pub fn internal_server_error_response() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR, 
        "An error has occurred. Please try again later."
    )
        .into_response()
}