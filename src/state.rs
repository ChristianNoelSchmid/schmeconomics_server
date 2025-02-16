use axum_macros::FromRef;
use schmeconomics_auth::auth_service::DynAuthService;
use tokens_rs::token_service::DynTokenService;

use crate::{categories::DynCategoryService, transactions::DynTransactionService};

#[derive(Clone, FromRef)]
pub struct AppState {
    pub auth_svc: DynAuthService,
    pub token_svc: DynTokenService,
    pub cat_svc: DynCategoryService,
    pub tx_svc: DynTransactionService,
}