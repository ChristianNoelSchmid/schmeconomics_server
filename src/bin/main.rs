use std::fs;

use axum::Router;
use reqwest::Client;
use schmeconomics_auth::auth_service::CoreAuthService;
use schmeconomics_server::{auth, categories::{self, DbConnCategoryService}, config::Config, currency_conv_provider::PaikamaCurrencyConversionProvider, state::AppState, transactions::{self, DbConnTransactionService}};
use sea_orm::Database;
use tokens_rs::{password_hasher::Argon2PasswordHasher, token_service::HmacSha256TokenService};
use tower_http::trace::TraceLayer;
use utils_rs::{date_time_provider::CoreTimeProvider, env_provider::CoreEnvProvider};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let env_provider = CoreEnvProvider::new_dyn();
    let time_provider = CoreTimeProvider::new_dyn();

    let db_con_str = env_provider.get_var("DATABASE_URL")?;
    let db = Database::connect(db_con_str).await?;

    let config = serde_json::from_str::<Config>(&fs::read_to_string("config.json")?)?;
    let token_svc = HmacSha256TokenService::new_dyn(env_provider, time_provider.clone(), config.token_svc_config);
    let password_hasher = Argon2PasswordHasher::new_dyn();
    let auth_svc = CoreAuthService::new_dyn(db.clone(), token_svc.clone(), time_provider.clone(), password_hasher.clone());
    let cc_provider = PaikamaCurrencyConversionProvider::new_dyn(Client::new());
    let cat_svc = DbConnCategoryService::new_dyn(db.clone());
    let tx_svc = DbConnTransactionService::new_dyn(db, time_provider, cc_provider);
    let app_state = AppState { auth_svc, token_svc, cat_svc, tx_svc, };

    let hashed = password_hasher.hash_password("test-password").unwrap();
    println!("{}", hashed);

    let app = Router::new()
        .nest(
            "/api/v1", 
            Router::new()
                .nest("/auth", auth::routes::routes(app_state.clone()))
                .nest("/categories", categories::routes::routes(app_state.clone()))
                .nest("/transactions", transactions::routes::routes(app_state))
        )
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
