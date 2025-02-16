use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db_utils::Role;

#[derive(Serialize)]
pub struct AccountResponseModel {
    ///
    /// Unique ID of the account
    /// 
    account_id: Uuid,
    ///
    /// All users and their role in the account
    /// 
    users: Vec<AccountUserModel>,
    /// 
    /// Date/Time that the account will be deleted
    /// in the event that the account is queued for deletion.
    /// `None` if account is not queued for deletion.
    /// 
    delete_on: Option<NaiveDateTime>,
}

#[derive(Deserialize, Serialize)]
pub struct AccountUserModel {
    pub user_id: Uuid,
    pub role: Role,
}