use axum::{extract::FromRequestParts, http::header};
use lazy_static::lazy_static;
use regex::Regex;
use sea_orm::prelude::Uuid;

use crate::state::AppState;

use super::error::*;

lazy_static! {
    static ref AUTH_RE: Regex =
        Regex::new(r"^(?i)bearer\s+([\w-]+\.[\w-]+\.[\w-]+)$").unwrap();
}

pub struct AuthUser { pub id: Uuid, }

impl FromRequestParts<AppState> for AuthUser 
{
    type Rejection = Error;
    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &AppState) -> std::result::Result<Self, Self::Rejection> {
        let token = extract_token(parts)?;

        // TODO - inject audience properly
        let claims = state.token_svc.verify_access_token("schmeconomics", &token)?;
        Ok(AuthUser { id: Uuid::parse_str(&claims["user_id"]).unwrap(), })
    }
}

fn extract_token<'a>(parts: &'a mut axum::http::request::Parts) -> Result<String> {
    let auth_header = parts.headers.get(header::AUTHORIZATION);
    if let None = auth_header { return Err(Error::Unauthorized); }
    let auth_header = auth_header.unwrap();

    let contents = auth_header.to_str();
    if let Ok(contents) = contents {
        return if let Some(token) = AUTH_RE.captures(contents) {
            Ok(token[1].to_string())
        } else {
            Err(Error::ParseHeaderError)
        }
    } else {
        return Err(Error::ParseHeaderError);
    }
}