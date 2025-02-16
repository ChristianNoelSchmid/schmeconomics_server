use reqwest::header::SET_COOKIE;
use schmeconomics_auth::auth_service::error::Result;
use axum::{extract::{Path, State}, http::HeaderName, response::AppendHeaders, routing::{post, put}, Json, Router};
use schmeconomics_auth::auth_service::{models::LoginModel, DynAuthService};

use crate::state::AppState;

pub fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/refresh", put(refresh))  
        .with_state(app_state)
}

pub async fn login(
    State(auth_svc): State<DynAuthService>,
    Json(req): Json<LoginModel>
) -> Result<(AppendHeaders<[(HeaderName, String);1]>, String)> {
    let tokens = auth_svc.get_access_token(&["schmeconomics"], &req.email, &req.password).await?;
    let headers = axum::response::AppendHeaders([
        (SET_COOKIE, format!("refresh-token={}", tokens.refr_token.contents))
    ]);

    Ok((headers, tokens.access_token.contents))
}

pub async fn refresh(
    State(auth_svc): State<DynAuthService>,
    Path(refr_token): Path<String>,
) -> Result<(AppendHeaders<[(HeaderName, String);1]>, String)> {
    let tokens = auth_svc.refresh(&refr_token).await?;
    let headers = axum::response::AppendHeaders([
        (SET_COOKIE, format!("refresh-token={}", tokens.refr_token.contents))
    ]);

    Ok((headers, tokens.access_token.contents))

}