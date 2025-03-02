use serde::Deserialize;
use tokens_rs::token_service::config::TokenServiceConfig;

use crate::validations;

#[derive(Deserialize)]
pub struct Config {
    pub token_svc_config: TokenServiceConfig,
    pub validation_svc_config: validations::Config,
}