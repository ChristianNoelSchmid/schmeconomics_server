use axum::{extract::State, routing::{delete, post}, Json, Router};

use crate::{auth::middleware::AuthUser, state::AppState};

use super::{error::Result, models::{CreateTransactionsModel, DeleteTransactionsModel, GetTransactionReqModel, TransactionModel}, DynTransactionService};

pub fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/query", post(get_transactions))
        .route("/create", post(post_transactions))  
        .route("/delete", delete(delete_transactions))
        .with_state(app_state)
}

pub async fn get_transactions(
    State(tx_svc): State<DynTransactionService>,
    user: AuthUser,
    Json(req): Json<GetTransactionReqModel>,
) -> Result<Json<Vec<TransactionModel>>> {
    Ok(Json(tx_svc.get_transactions(user.id, req).await?))
}

pub async fn post_transactions(
    State(tx_svc): State<DynTransactionService>,
    user: AuthUser,
    Json(body): Json<CreateTransactionsModel>,
) -> Result<()> {
    tx_svc.create_transactions(user.id, body).await?;
    Ok(())
}

pub async fn delete_transactions(
    State(tx_svc): State<DynTransactionService>,
    user: AuthUser,
    Json(body): Json<DeleteTransactionsModel>
) -> Result<()> {
    tx_svc.delete_transactions(user.id, body).await?;
    Ok(())
}