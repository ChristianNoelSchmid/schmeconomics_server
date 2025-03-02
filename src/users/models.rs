use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateUserRequestModel {
    pub email: String,
    pub password: String,
    pub name: String,
    pub two_factor_auth: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequestModel {
    pub email: Option<String>,
    pub password: Option<String>,
    pub name: Option<String>,
    pub two_factor_enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserResponseModel {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub name: String,
    pub two_factor_enabled: bool,
}