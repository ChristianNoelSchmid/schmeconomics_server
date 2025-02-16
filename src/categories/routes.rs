use axum::{extract::{Path, State}, routing::{delete, get, post, put}, Json, Router};
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, categories::Result, state::AppState};

use super::{models::DeleteCategoryModel, CreateCategoryModel, DynCategoryService, GetCategoryModel, UpdateCategoryModel};

pub fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/{account_id}", get(get_categories))
        .route("/", post(post_category))  
        .route("/", put(update_category))
        .route("/", delete(delete_category))
        .with_state(app_state)
}

pub async fn get_categories(
    State(cat_svc): State<DynCategoryService>,
    Path(account_id): Path<Uuid>,
    user: AuthUser,
) -> Result<Json<Vec<GetCategoryModel>>> {
    Ok(Json(cat_svc.get_cats(user.id, account_id).await?))
}

pub async fn post_category(
    State(cat_svc): State<DynCategoryService>,
    user: AuthUser,
    Json(body): Json<CreateCategoryModel>,
) -> Result<Json<GetCategoryModel>> {
    Ok(Json(cat_svc.create_cat(user.id, body).await?))
}

pub async fn update_category(
    State(cat_svc): State<DynCategoryService>,
    user: AuthUser,
    Json(body): Json<UpdateCategoryModel>,
) -> Result<Json<GetCategoryModel>> {
    Ok(Json(cat_svc.update_cat(user.id, body).await?))
}

pub async fn delete_category(
    State(cat_svc): State<DynCategoryService>,
    user: AuthUser,
    Json(body): Json<DeleteCategoryModel>
) -> Result<()> {
    cat_svc.delete_cat(user.id, body).await?;
    Ok(())
}