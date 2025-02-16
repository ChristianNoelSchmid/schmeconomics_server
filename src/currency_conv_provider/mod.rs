use std::sync::Arc;

use async_trait::async_trait;

#[cfg(test)]
use mockall::automock;

use error::*;
use models::ResponseModel;
use reqwest::Client;

pub mod error;
pub mod models;

const PAIKAMA_BASE_URL: &'static str = "https://hexarate.paikama.co/api/rates/latest/";
pub const USD_CURRENCY_TYPE: &'static str = "USD";

pub type DynCurrencyConversionProvider = Arc<dyn CurrencyConversionProvider + Send + Sync>;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait CurrencyConversionProvider {
    async fn convert(&self, from_type: &str, to_type: &str, am: i64) -> Result<i64> {
        if from_type == to_type { return Ok(am); }
        let conv = self.get_conversion(&from_type, &to_type).await?;
        Ok(f64::floor(conv * am as f64) as i64)
    }
    async fn get_conversion(&self, from_currency_type: &str, to_currency_type: &str) -> Result<f64>;
}

pub struct PaikamaCurrencyConversionProvider {
    client: Client
}

impl PaikamaCurrencyConversionProvider {
    pub fn new_dyn(client: Client) -> DynCurrencyConversionProvider {
        Arc::new(PaikamaCurrencyConversionProvider { client })
    }
}

#[async_trait]
impl CurrencyConversionProvider for PaikamaCurrencyConversionProvider {
    async fn get_conversion(&self, from_currency_type: &str, to_currency_type: &str) -> Result<f64> {
        let res = self.client.get(
            format!("{}/{}?target={}", PAIKAMA_BASE_URL, from_currency_type, to_currency_type)
        ).send().await?;

        if !res.status().is_success() {
            return Err(Error::StatusCodeFetchError(res.status(), res.text().await.unwrap_or(String::new())));
        }

        let body = serde_json::from_str::<ResponseModel>(&res.text().await?)?;
        Ok(body.data.mid)
    }
}