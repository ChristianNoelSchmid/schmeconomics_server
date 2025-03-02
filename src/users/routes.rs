use axum::{extract::State, routing::{delete, get, post, put}, Json, Router};

use crate::{auth::middleware::AuthUser, state::AppState};

use super::{error::Result, models::{CreateUserRequestModel, UpdateUserRequestModel, UserResponseModel}, DynUserService};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/get", get(get_user))
        .route("/create", post(create_user))
        .route("/update", put(update_user))
        .route("/delete", delete(delete_user))
        .with_state(state)
}

async fn get_user(
    user: AuthUser,
    State(user_svc): State<DynUserService>,
) -> Result<Json<UserResponseModel>> {
    Ok(Json(user_svc.get_user(user.id).await?))
}

async fn create_user(
    State(user_svc): State<DynUserService>,
    Json(body): Json<CreateUserRequestModel>,
) -> Result<Json<UserResponseModel>> {
    Ok(Json(user_svc.create_user(body).await?))
}

async fn update_user(
    user: AuthUser,
    State(user_svc): State<DynUserService>,
    Json(body): Json<UpdateUserRequestModel>,
) -> Result<Json<UserResponseModel>> {
    Ok(Json(user_svc.update_user(user.id, body).await?))
}

async fn delete_user(
    user: AuthUser,
    State(user_svc): State<DynUserService>,
) -> Result<()> {
    user_svc.delete_user(user.id).await?;
    Ok(())
}