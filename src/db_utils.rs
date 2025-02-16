use std::str::FromStr;

use schmeconomics_entities::prelude::AccountUsers;
use sea_orm::{ConnectionTrait, EntityTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub enum Role { Read, Write, Admin, }
impl FromStr for Role {
    type Err = DbUtilsError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Read" => Ok(Self::Read),
            "Write" => Ok(Self::Write),
            "Admin" => Ok(Self::Admin),
            _ => Err(DbUtilsError::CouldNotParseRole(s.to_string())),
        }
    }
}

impl ToString for Role {
    fn to_string(&self) -> String {
        match self {
            Self::Read => String::from("Read"),
            Self::Write => String::from("Write"),
            Self::Admin => String::from("Admin"),
        }
    }
}

pub async fn validate_user_owns_account(tx: &impl ConnectionTrait, user_id: Uuid, account_id: Uuid) -> Result<(), DbUtilsError> {
    return if let None = AccountUsers::find_by_id((account_id, user_id)).one(tx).await? {
        Err(DbUtilsError::UserNotPartOfAccount(user_id, account_id))
    } else {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DbUtilsError {
    #[error("Database error occurred: {0}")]
    DbErr(#[from] sea_orm::DbErr),
    #[error("User {0} is not part of account {1}")]
    UserNotPartOfAccount(Uuid, Uuid),
    #[error("Could not parse Role from string {0}")]
    CouldNotParseRole(String),
}