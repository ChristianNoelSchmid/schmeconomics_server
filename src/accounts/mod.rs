pub mod error;
pub mod models;

use async_trait::async_trait;

use chrono::NaiveDateTime;
use error::*;
use models::{AccountResponseModel, AccountUserModel};
use sea_orm::DbConn;
use uuid::Uuid;

#[async_trait]
pub trait AccountService {
    async fn create_account(&self, users: Vec<AccountUserModel>) -> Result<AccountResponseModel>;
    async fn update_account(&self, account_id: Uuid, users: Vec<AccountUserModel>) -> Result<AccountResponseModel>;
    ///
    /// Deletes an account, performed by user with the given `user_id`.
    /// Allows temporary delete-state of account in event that users wish to undo the operation.
    /// Returns the date the account will truly be deleted.
    /// 
    async fn delete_account(&self, user_id: Uuid, account_id: Uuid) -> Result<NaiveDateTime>;
    async fn get_account(&self, user_id: Uuid, account_id: Uuid) -> Result<AccountResponseModel>;
}

pub struct DbConnAccountService {
    db: DbConn,
}

#[async_trait]
impl AccountService for DbConnAccountService {
    async fn create_account(&self, users: Vec<AccountUserModel>) -> Result<AccountResponseModel> {
        todo!()
    }
    async fn update_account(&self, account_id: Uuid, users: Vec<AccountUserModel>) -> Result<AccountResponseModel> {
        todo!()
    }
    async fn delete_account(&self, user_id: Uuid, account_id: Uuid) -> Result<NaiveDateTime> {
        todo!()
    }
    async fn get_account(&self, user_id: Uuid, account_id: Uuid) -> Result<AccountResponseModel> {
        todo!()
    }
}