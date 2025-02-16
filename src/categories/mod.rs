use std::{collections::HashSet, sync::Arc};

use async_trait::async_trait;
use schmeconomics_entities::{categories, prelude::*};
use sea_orm::{prelude::{Expr, Uuid}, sea_query::{ExprTrait, Func}, ActiveValue::NotSet, ColumnTrait, Condition, ConnectionTrait, DbConn, EntityTrait, FromQueryResult, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait};

use crate::db_utils::validate_user_owns_account;

use {error::*, models::*};

pub mod models;
pub mod error;

#[cfg(test)]
mod test;
pub mod routes;

pub type DynCategoryService = Arc<dyn CategoryService + Send + Sync>;

#[async_trait]
pub trait CategoryService {
    async fn get_cats(&self, user_id: Uuid, account_id: Uuid) -> Result<Vec<GetCategoryModel>>;
    async fn create_cat(&self, user_id: Uuid, cat: CreateCategoryModel) -> Result<GetCategoryModel>;
    async fn update_cat(&self, user_id: Uuid, cat: UpdateCategoryModel) -> Result<GetCategoryModel>;
    async fn delete_cat(&self, user_id: Uuid, cat: DeleteCategoryModel) -> Result<()>;
    async fn order_cats(&self, user_id: Uuid, cats: OrderCategoriesModel) -> Result<()>;
}

pub struct DbConnCategoryService {
    db: DbConn,
}

#[async_trait]
impl CategoryService for DbConnCategoryService {
    async fn get_cats(&self, user_id: Uuid, account_id: Uuid) -> Result<Vec<GetCategoryModel>> {
        validate_user_owns_account(&self.db, user_id, account_id).await?;

        let cats = Categories::find().filter(categories::Column::AccountId.eq(account_id))
            .order_by_asc(categories::Column::Order)
            .all(&self.db).await?;

        Ok(
            cats.into_iter().map(
                |cat| GetCategoryModel { 
                    id: cat.id, 
                    name: cat.name, 
                    balance: cat.balance,
                    refill_val: cat.refill_value,
                }
            )
                .collect()
        )
    }
    async fn create_cat(&self, user_id: Uuid, create_cat: CreateCategoryModel) -> Result<GetCategoryModel> {
        validate_user_owns_account(&self.db, user_id, create_cat.account_id).await?;

        // Create new transaction
        let tx = self.db.begin().await?;

        // Remove whitespacing from the cat name
        let fmt_cat_name = create_cat.name.trim().to_string();
        // Validate the category name
        self.validate_cat_name(create_cat.account_id, &fmt_cat_name, &tx).await?;

        // Get the highest order value for the account currently in the database
        #[derive(FromQueryResult)]
        struct MaxOrderQuery { max: Option<i32> }
        let max_order = Categories::find().select_only()
            .filter(categories::Column::AccountId.eq(create_cat.account_id))
            .column_as(categories::Column::Order.max(), "max")
            .into_model::<MaxOrderQuery>().one(&tx)
            .await?.unwrap().max.unwrap_or(0);

        let new_id = uuid::Uuid::now_v7();

        let new_cat = categories::ActiveModel {
            id: Set(new_id),
            account_id: Set(create_cat.account_id),
            name: Set(fmt_cat_name.to_string()), 
            balance: Set(create_cat.init_bal), 
            refill_value: Set(create_cat.refill_val), 
            order: Set(max_order + 1),
        };

        Categories::insert(new_cat).exec(&tx).await?;
        tx.commit().await?;

        Ok(
            GetCategoryModel { 
                id: new_id,
                name: fmt_cat_name,
                balance: create_cat.init_bal,
                refill_val: create_cat.refill_val,
            }
        )
    }
    async fn update_cat(&self, user_id: Uuid, cat: UpdateCategoryModel) -> Result<GetCategoryModel> {
        validate_user_owns_account(&self.db, user_id, cat.account_id).await?;

        let tx = self.db.begin().await?;
        // Create a formatted category name,
        // checking if the name already exists in the collection
        let fmt_cat_name = if let Some(unfmt_cat_name) = cat.new_name {
            let res = unfmt_cat_name.trim().to_string();
            // Validate the update name
            self.validate_cat_name(cat.account_id, &res, &tx).await?;
            Some(res)
        } else {
            None
        };

        // Find the category to update
        let ex_cat = Categories::find_by_id(cat.id)
            .filter(categories::Column::AccountId.eq(cat.account_id))
            .one(&tx).await?;

        return if let Some(ex_cat) = ex_cat {
            // Update the row with each value provided
            let mut ex_cat = ex_cat.into_active_model();
            ex_cat.name = if let Some(fmt_cat_name) = fmt_cat_name { Set(fmt_cat_name) } else { NotSet };
            ex_cat.refill_value = if let Some(refill_val) = cat.new_refill_val { Set(refill_val) } else { NotSet };
            ex_cat.balance = if let Some(bal) = cat.new_bal { Set(bal) } else { NotSet };
            let updated = Categories::update(ex_cat).exec(&tx).await?;
            tx.commit().await?;

            Ok(GetCategoryModel {
                id: updated.id,
                name: updated.name,
                balance: updated.balance,
                refill_val: updated.refill_value,
            })
        } else {
            // Return Err if the category is not found for the account
            Err(Error::CategoryNotFound(cat.id))
        };
    }

    async fn delete_cat(&self, user_id: Uuid, delete_cat: DeleteCategoryModel) -> Result<()> {
        validate_user_owns_account(&self.db, user_id, delete_cat.account_id).await?;

        // Create a new transaction
        let tx = self.db.begin().await?;
        // Find the category to delete in the given account
        let cat = Categories::find_by_id(delete_cat.cat_id)
            .filter(categories::Column::AccountId.eq(delete_cat.account_id))
            .one(&tx).await?;

        return if let Some(cat) = cat {  
            Categories::update_many().filter(categories::Column::Order.gt(cat.order))
                .col_expr(categories::Column::Order, Expr::col(categories::Column::Order).add(1))
                .exec(&tx).await?;
            Categories::delete(cat.into_active_model()).exec(&tx).await?;
            tx.commit().await?;
            
            Ok(())
        } else {
            // Return Err if the category is not found for the account
            Err(Error::CategoryNotFound(delete_cat.cat_id))
        };
    }
    async fn order_cats(&self, user_id: Uuid, cats: OrderCategoriesModel) -> Result<()> {
        validate_user_owns_account(&self.db, user_id, cats.account_id).await?;

        let mut ord_set = HashSet::new();
        let mut id_set = HashSet::new();
        let tx = self.db.begin().await?;
        for (id, ord) in cats.orders.iter() {
            if !ord_set.insert(ord) {
                return Err(Error::OrderDuplicateIndex(*ord));
            }
            if !id_set.insert(id) {
                return Err(Error::OrderDuplicateId(*id));
            }

            let update_cat = Categories::find_by_id(*id)
                .filter(categories::Column::AccountId.eq(cats.account_id))
                .one(&tx).await?;

            if let Some(update_cat) = update_cat {
                let mut update_cat = update_cat.into_active_model();
                update_cat.order = Set(*ord);
                Categories::update(update_cat).exec(&tx).await?;
            } else {
                return Err(Error::CategoryNotFound(*id));
            }
        }
        tx.commit().await?;
        Ok(()) 
    }
}

impl DbConnCategoryService {
    pub fn new_dyn(db: DbConn) -> DynCategoryService {
        Arc::new(DbConnCategoryService { db })
    }
    async fn validate_cat_name(
        &self, 
        account_id: Uuid, 
        cat_name: &str, 
        tx: &impl ConnectionTrait
    ) -> Result<()> {
        // Check if there is a category that already exists
        // in the account, with the provided name
        let existing_cat = Categories::find().filter(
            Condition::all()
                .add(categories::Column::AccountId.eq(account_id))
                .add(Func::lower(Expr::col(categories::Column::Name)).eq(cat_name.to_lowercase()))
            ).one(tx).await?;

        // If a category matches, return NameReuse error
        if existing_cat.is_some() {
            return Err(Error::NameReuse(cat_name.to_string()));
        }

        Ok(())
    }    
}