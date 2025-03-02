use axum::{extract::{Path, State}, routing::{delete, get, post, put}, Json, Router};
use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, state::AppState};

use super::{error::Result, models::{AccountInfoResponseModel, AccountResponseModel, AccountUserModel, CreateAccountRequestModel}, DynAccountService};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/all", get(get_account_infos))
        .route("/create", post(create_account))
        .route("/{account_id}", get(get_account))
        .route("/{account_id}/upsert-user", put(upsert_user))
        .route("/{account_id}/delete", delete(delete_account))
        .route("/{account_id}/delete-user/{user_id}", delete(delete_user_from_account))
        .with_state(state) 
}

async fn get_account_infos(
    user: AuthUser,
    State(account_svc): State<DynAccountService>,
) -> Result<Json<Vec<AccountInfoResponseModel>>> {
    Ok(Json(account_svc.get_account_infos(user.id).await?))
}

async fn get_account(
    user: AuthUser,
    State(account_svc): State<DynAccountService>,
    Path(account_id): Path<Uuid>
) -> Result<Json<AccountResponseModel>> {
    Ok(Json(account_svc.get_account(user.id, account_id).await?))
}

async fn create_account(
    user: AuthUser,
    State(account_svc): State<DynAccountService>,
    Json(body): Json<CreateAccountRequestModel>,
) -> Result<Json<AccountResponseModel>> {
    Ok(Json(account_svc.create_account(user.id, body).await?))
}

async fn upsert_user(
    user: AuthUser,
    State(account_svc): State<DynAccountService>,
    Path(account_id): Path<Uuid>,
    Json(body): Json<AccountUserModel>
) -> Result<()> {
    Ok(account_svc.upsert_user_account(account_id, user.id, body).await?)
}

async fn delete_account(
    user: AuthUser,
    Path(account_id): Path<Uuid>,
    State(account_svc): State<DynAccountService>,
) -> Result<Json<NaiveDateTime>> {
    Ok(Json(account_svc.delete_account(user.id, account_id).await?)) 
}

async fn delete_user_from_account(
    user: AuthUser,
    Path(account_id): Path<Uuid>,
    Path(user_id): Path<Uuid>,
    State(account_svc): State<DynAccountService>,
) -> Result<()> {
    Ok(account_svc.remove_user_from_account(user.id, account_id, user_id).await?)
}