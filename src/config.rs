use serde::Deserialize;
use tokens_rs::token_service::config::TokenServiceConfig;

#[derive(Deserialize)]
pub struct Config {
    pub token_svc_config: TokenServiceConfig
}