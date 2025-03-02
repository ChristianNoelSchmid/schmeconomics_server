pub mod error;
pub mod models;
pub mod routes;

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use chrono::{Days, NaiveDateTime};
use error::*;
use models::{AccountInfoResponseModel, AccountResponseModel, AccountUserModel, CreateAccountRequestModel};
use schmeconomics_entities::{account_users, accounts, prelude::{AccountUsers, Accounts, Users}, users};
use sea_orm::{prelude::Expr, ColumnTrait, DbConn, EntityTrait, IntoActiveModel, QueryFilter, Set, TransactionTrait};
use send_email_rs::{models::EmailModel, DynSendEmailService};
use tera::Context;
use utils_rs::date_time_provider::DynDateTimeProvider;
use uuid::Uuid;

use crate::{db_utils::{validate_user_account_role, Role, ValidationContext}, validations::DynValidationService};

pub type DynAccountService = Arc<dyn AccountService + Send + Sync>;

#[async_trait]
pub trait AccountService {
    async fn create_account(&self, user_id: Uuid, req: CreateAccountRequestModel) -> Result<AccountResponseModel>;
    async fn get_account_infos(&self, user_id: Uuid) -> Result<Vec<AccountInfoResponseModel>>;
    async fn get_account(&self, user_id: Uuid, account_id: Uuid) -> Result<AccountResponseModel>;
    ///
    /// Deletes an account, performed by user with the given `user_id`.
    /// Allows temporary delete-state of account in event that users wish to undo the operation.
    /// Returns the date the account will truly be deleted.
    /// 
    async fn delete_account(&self, admin_user_id: Uuid, account_id: Uuid) -> Result<NaiveDateTime>;
    ///
    /// Adds a single user to the account, with the specified Role
    /// 
    async fn upsert_user_account(
        &self, 
        account_id: Uuid, 
        admin_user_id: Uuid,
        req: AccountUserModel,
    ) -> Result<()>;
    async fn remove_user_from_account(&self, admin_user_id: Uuid, account_id: Uuid, user_id: Uuid) -> Result<()>;
}

pub struct DbConnAccountService {
    db: DbConn,
    send_email_svc: DynSendEmailService<Context>,
    validation_svc: DynValidationService,
    dt_provider: DynDateTimeProvider,
}

#[async_trait]
impl AccountService for DbConnAccountService {
    async fn create_account(&self, user_id: Uuid, req: CreateAccountRequestModel) -> Result<AccountResponseModel> {
        let tx = self.db.begin().await?;
        let new_id = Uuid::now_v7();
        let new_account = accounts::ActiveModel {
            id: Set(new_id),

            ..Default::default()
        };

        Accounts::insert(new_account).exec(&tx).await?;

        let mut new_account_users = vec![];
        let user_ids = req.users.iter().map(|u| u.user_id);
        let user_by_ids = Users::find().filter(users::Column::Id.is_in(user_ids)).all(&tx).await?
            .into_iter().map(|u| (u.id, u)).collect::<HashMap<Uuid, users::Model>>();

        for user in &req.users {
            if !user_by_ids.contains_key(&user.user_id) {
                return Err(Error::UserNotFound(user.user_id));
            }
            let new_account_user = account_users::ActiveModel {
                account_id: Set(new_id), 
                user_id: Set(user.user_id), 
                role: Set(user.role.to_string()),

                ..Default::default()
            };

            new_account_users.push(new_account_user);
            let token = self.validation_svc.add_validation(
                ValidationContext::AddAccount { account_id: new_id, user_id }
            ).await?;

            self.send_verify_email(
                &user_by_ids[&user.user_id].email, 
                user.user_id, 
                &user_by_ids[&user.user_id].name, 
                &token,
            ).await?;
        }

        AccountUsers::insert_many(new_account_users).exec(&tx).await?;
        tx.commit().await?;

        Ok(
            AccountResponseModel { 
                account_id: new_id, 
                name: req.name,
                users: req.users, 
                delete_on: None, 
            }
        )
    }
    async fn upsert_user_account(
        &self, 
        account_id: Uuid, 
        admin_user_id: Uuid,
        req: AccountUserModel
    ) -> Result<()> {
        let tx = self.db.begin().await?;
        validate_user_account_role(&tx, admin_user_id, account_id, Role::Admin).await?;

        let user = Users::find_by_id(req.user_id)
            .find_also_related(account_users::Entity).one(&tx).await?;

        match user {
            Some((user, None)) => {
                let new_account_user = account_users::ActiveModel {
                    account_id: Set(account_id), 
                    user_id: Set(req.user_id), 
                    role: Set(req.role.to_string()),

                    ..Default::default()
                };           
                AccountUsers::insert(new_account_user).exec(&tx).await?;
                let token = self.validation_svc.add_validation(
                    ValidationContext::AddAccount { account_id, user_id: req.user_id }
                ).await?;
                self.send_verify_email(&user.email, user.id, &user.name, &token).await?;

                tx.commit().await?;

                Ok(())
            },
            Some((_, Some(account_user))) => { 
                let mut account_user = account_user.into_active_model();
                account_user.role = Set(req.role.to_string());
                AccountUsers::update(account_user).exec(&tx).await?;

                tx.commit().await?;

                Ok(())
            },
            None => {
                Err(Error::UserNotFound(req.user_id))
            },
        }
    }
    async fn remove_user_from_account(&self, admin_user_id: Uuid, account_id: Uuid, user_id: Uuid) -> Result<()> {
        let tx = self.db.begin().await?;
        validate_user_account_role(&tx, admin_user_id, account_id, Role::Admin).await?;
        let res = AccountUsers::delete_by_id((account_id, user_id)).exec(&self.db).await?;

        return if res.rows_affected > 0 {
            tx.commit().await?;
            Ok(())
        } else {
            Err(Error::AccountUserNotFound(account_id, user_id))
        };
    }
    async fn delete_account(&self, admin_user_id: Uuid, account_id: Uuid) -> Result<NaiveDateTime> {
        let tx = self.db.begin().await?;
        validate_user_account_role(&tx, admin_user_id, account_id, Role::Admin).await?;
        let expires_at = self.dt_provider.utc_now().checked_add_days(Days::new(30)).unwrap();

        let res = Accounts::update_many().filter(accounts::Column::Id.eq(account_id))
            .col_expr(accounts::Column::DeleteOn, Expr::value(expires_at))
            .exec(&tx).await?;

        return if res.rows_affected > 0 {
            tx.commit().await?;
            Ok(expires_at.naive_utc())
        } else {
            Err(Error::AccountNotFound(account_id))
        }
    }
    async fn get_account(&self, user_id: Uuid, account_id: Uuid) -> Result<AccountResponseModel> {
        let tx = self.db.begin().await?;
        validate_user_account_role(&tx, user_id, account_id, Role::Read).await?;

        let account = Accounts::find_by_id(account_id).one(&tx).await?.unwrap();
        let account_users = AccountUsers::find()
            .filter(account_users::Column::AccountId.eq(account_id))
            .all(&tx).await?;

        Ok(AccountResponseModel { 
            account_id, 
            name: account.name,
            users: account_users.into_iter().map(|u| AccountUserModel { 
                user_id: u.user_id, 
                role: u.role.parse::<Role>().unwrap()
            }).collect(), 
            delete_on: account.delete_on.and_then(|d| Some(d.naive_utc()))
        })
    }
    async fn get_account_infos(&self, user_id: Uuid) -> Result<Vec<AccountInfoResponseModel>> {

        // Create the transaction for the request
        let tx = self.db.begin().await?;

        // Get all account_users matching the provided user ID
        // Joined to their matching accounts
        let user_accounts = AccountUsers::find()
            .filter(account_users::Column::UserId.eq(user_id))
            .find_also_related(Accounts)
            .all(&tx).await?;

        Ok(
            user_accounts.into_iter().map(
                |ua| AccountInfoResponseModel { 
                    id: ua.0.account_id, 
                    name: ua.1.unwrap().name 
                }
            )
                .collect()
        )
    }
}

impl DbConnAccountService {
    pub fn new_dyn(
        db: DbConn,
        send_email_svc: DynSendEmailService<Context>,
        validation_svc: DynValidationService,
        dt_provider: DynDateTimeProvider,
    ) -> DynAccountService {
        Arc::new(Self { db, send_email_svc, validation_svc, dt_provider })
    }
    async fn send_verify_email(
        &self, email: &str, user_id: Uuid, name: &str, token: &str
    ) -> Result<()> {

        let mut ctx = Context::new();
        ctx.insert("token", token);
        ctx.insert("user_id", &user_id);
        ctx.insert("name", name);

        self.send_email_svc.send_email(
            EmailModel {
                from_email_addr: "chris@christianssoftware.com",
                to_email_addr: &email,
                subject: "Verify email address",
            }, 
            "verify_email.html",
            &ctx
        ).await?;

        Ok(())
    }
}