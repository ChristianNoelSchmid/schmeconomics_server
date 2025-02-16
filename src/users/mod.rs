pub mod error;
pub mod models;

use std::sync::Arc;

use async_trait::async_trait;
use schmeconomics_entities::{prelude::Users, users};
use sea_orm::{ActiveValue::NotSet, ColumnTrait, DbConn, EntityTrait, IntoActiveModel, QueryFilter, Set, TransactionTrait};
use tokens_rs::password_hasher::DynPasswordHasher;
use uuid::Uuid;

use {error::*, models::*};

pub type DynUserService = Arc<dyn UserService + Send + Sync>;

#[async_trait]
pub trait UserService {
    async fn create_user(&self, req: CreateUserRequestModel) -> Result<UserResponseModel>;
    async fn get_user(&self, user_id: Uuid) -> Result<UserResponseModel>;
    async fn delete_user(&self, user_id: Uuid) -> Result<()>;
    async fn update_user(&self, user_id: Uuid, req: UpdateUserRequestModel) -> Result<UserResponseModel>;
}

pub struct DbConnUserService {
    db: DbConn,
    password_hasher: DynPasswordHasher,
}

#[async_trait]
impl UserService for DbConnUserService {
    async fn create_user(&self, req: CreateUserRequestModel) -> Result<UserResponseModel> {
        let tx = self.db.begin().await?;

        // Check if there's a user that already has the existing email before continuing
        let fmt_email = req.email.trim().to_lowercase();
        let existing_user = Users::find()
            .filter(users::Column::Email.eq(fmt_email.clone()))
            .one(&tx).await?;

        if existing_user.is_some() {
            return Err(Error::EmailInUse(fmt_email));
        }

        let password_hash = self.password_hasher.hash_password(&req.password)?;
        let new_id = Uuid::now_v7();

        let new_user = users::ActiveModel { 
            id: Set(new_id), 
            email: Set(req.email), 
            password_hash: Set(password_hash), 
            name: Set(req.name.clone()), 

            ..Default::default()
        };
        Users::insert(new_user).exec(&tx).await?;

        tx.commit().await?;

        Ok(
            UserResponseModel { 
                id: new_id,
                email: fmt_email, 
                email_verified: false, 
                name: req.name, 
                two_factor_enabled: false
            }
        )
    }

    async fn get_user(&self, user_id: Uuid) -> Result<UserResponseModel> {
        let user = Users::find_by_id(user_id).one(&self.db).await?;
        return match user {
            Some(user) => Ok(UserResponseModel { 
                id: user.id, 
                email: user.email, 
                email_verified: user.email_verified, 
                name: user.name, 
                two_factor_enabled: user.two_factor_enabled ,
            }),
            None => Err(Error::UserNotFound(user_id)),
        };
    }

    async fn delete_user(&self, user_id: Uuid) -> Result<()> {
        let user = Users::find_by_id(user_id).one(&self.db).await?;
        return match user {
            Some(user) => {
                Users::delete(user.into_active_model()).exec(&self.db).await?;
                Ok(())
            },
            None => Err(Error::UserNotFound(user_id)),
        };
    }

    async fn update_user(&self, user_id: Uuid, req: UpdateUserRequestModel) -> Result<UserResponseModel> {
        let tx = self.db.begin().await?;
        let user = Users::find_by_id(user_id).one(&tx).await?;
        return if let Some(mut user) = user.and_then(|u| Some(u.into_active_model())) {
            (user.email, user.email_verified) = if let Some(email) = req.email { 
                (Set(email), Set(false))
            } else { 
                (NotSet, NotSet)
            };
            user.password_hash = if let Some(pwd) = req.password { 
                let password_hash = self.password_hasher.hash_password(&pwd)?;
                Set(password_hash) 
            } else { 
                NotSet 
            };
            user.name = if let Some(name) = req.name { Set(name) } else { NotSet };
            user.two_factor_enabled = if let Some(tfe) = req.two_factor_enabled { Set(tfe) } else { NotSet };

            let user = Users::update(user).exec(&tx).await?;
            tx.commit().await?;

            Ok(
                UserResponseModel {
                    id: user.id, 
                    email: user.email, 
                    email_verified: user.email_verified,
                    name: user.name, 
                    two_factor_enabled: user.two_factor_enabled, 
                }
            )
        } else {
            Err(Error::UserNotFound(user_id))
        }
    }
}

impl DbConnUserService {
    fn new_dyn(db: DbConn, password_hasher: DynPasswordHasher) -> Self {
        Self { db, password_hasher, }
    }
}