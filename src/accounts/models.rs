use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db_utils::Role;

#[derive(Deserialize)]
pub struct CreateAccountRequestModel {
    pub name: String,
    pub users: Vec<AccountUserModel>,
}

#[derive(Serialize)]
pub struct AccountInfoResponseModel {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize)]
pub struct AccountResponseModel {
    ///
    /// Unique ID of the account
    /// 
    pub account_id: Uuid,
    ///
    /// The name of the account
    /// 
    pub name: String,
    ///
    /// All users and their role in the account
    /// 
    pub users: Vec<AccountUserModel>,
    /// 
    /// Date/Time that the account will be deleted
    /// in the event that the account is queued for deletion.
    /// `None` if account is not queued for deletion.
    /// 
    pub delete_on: Option<NaiveDateTime>,
}

#[derive(Deserialize, Serialize)]
pub struct AccountUserModel {
    pub user_id: Uuid,
    pub role: Role,
}