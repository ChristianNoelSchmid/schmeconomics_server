pub mod error;

use std::sync::Arc;
use async_trait::async_trait;
use chrono::Duration;
use error::*;
#[cfg(test)]
use mockall::automock;
use schmeconomics_entities::{account_users, prelude::{AccountUsers, Users, Validations}, users, validations};
use sea_orm::{prelude::Expr, ColumnTrait, DbConn, EntityTrait, IntoActiveModel, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use tokens_rs::token_service::DynTokenService;
use utils_rs::date_time_provider::DynDateTimeProvider;
use uuid::Uuid;

use crate::db_utils::{ValidationContext, ValidationKind};

pub type DynValidationService = Arc<dyn ValidationService + Send + Sync>;

#[derive(Deserialize)]
pub struct Config {
    pub verify_email_lt_s: i64,
    pub add_account_lt_s: i64,
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait ValidationService {
    async fn add_validation(&self, kind: ValidationContext,) -> Result<String>;
    async fn validate(&self, kind: ValidationKind, token: String,) -> Result<()>;
}

pub struct DbConnValidationService {
    db: DbConn,
    token_svc: DynTokenService,
    dt_provider: DynDateTimeProvider,
    config: Config,
}

#[async_trait]
impl ValidationService for DbConnValidationService {
    async fn add_validation(&self, ctx: ValidationContext,) -> Result<String> {
        // Generate an expiration timestamp
        let now = self.dt_provider.utc_now().checked_add_signed(
            Duration::seconds(
                match ctx {
                    ValidationContext::VerifyEmail { user_id: _ } 
                        => self.config.verify_email_lt_s,
                    ValidationContext::AddAccount { account_id: _, user_id: _ } 
                        => self.config.add_account_lt_s
                }
            )
        )
            .expect("Could not add seconds to UTC now - check ValidationService config");

        // Generate a random token
        let new_id = Uuid::now_v7();
        let token = self.token_svc.generate_random_bytes(16);
        let context = serde_json::to_string(&ctx).unwrap();

        let tx = self.db.begin().await?;
        let validation = validations::ActiveModel { 
            id: Set(new_id), 
            context: Set(context),
            token: Set(token.clone()),
            valid_until_utc: Set(now),
        };
        Validations::insert(validation).exec(&tx).await?;

        tx.commit().await?;

        // Return the new ID
        Ok(token)
    }

    async fn validate(&self, kind: ValidationKind, token: String,) -> Result<()> {
        let tx = self.db.begin().await?;
        let validation = Validations::find().filter(validations::Column::Token.eq(&token))
            .one(&tx).await?;

        if let Some(validation) = validation {
            let ctx: ValidationContext = serde_json::from_str(&validation.context)?;
            if self.dt_provider.utc_now() > validation.valid_until_utc {
                return Err(Error::ValidationExpired(token));
            }

            match (&kind, &ctx) {
                (ValidationKind::VerifyEmail, ValidationContext::VerifyEmail { user_id }) => {
                    Users::update_many()
                        .filter(users::Column::Id.eq(*user_id))
                        .col_expr(users::Column::EmailVerified, Expr::value(true))
                        .exec(&tx).await?;
                },
                (ValidationKind::AddAccount, ValidationContext::AddAccount { account_id, user_id }) => {
                    AccountUsers::update_many()
                        .filter(account_users::Column::UserId.eq(*user_id))
                        .filter(account_users::Column::AccountId.eq(*account_id))
                        .col_expr(account_users::Column::Verified, Expr::value(true))
                        .exec(&tx).await?;
                },
                (_, _) => {
                    return Err(Error::MismatchedValidation(kind, ctx));
                }
            }

            Validations::delete(validation.into_active_model()).exec(&tx).await?;
            return Ok(())
        }
        Err(Error::ValidationNotFound(token))
    }
}

impl DbConnValidationService {
    pub fn new_dyn(
        db: DbConn,
        token_svc: DynTokenService,
        dt_provider: DynDateTimeProvider,
        config: Config,
    ) -> DynValidationService {
        Arc::new(Self { db, token_svc, dt_provider, config })
    }
}