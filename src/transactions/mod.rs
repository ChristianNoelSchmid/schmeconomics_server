use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use sea_orm::{prelude::{Expr, Uuid}, ColumnTrait, DbConn, EntityTrait, PaginatorTrait, QueryFilter, Set, TransactionTrait};

use schmeconomics_entities::{categories, prelude::*, transactions};
use utils_rs::date_time_provider::DynDateTimeProvider;

use crate::{currency_conv_provider::{DynCurrencyConversionProvider, USD_CURRENCY_TYPE}, db_utils::{validate_user_account_role, Role}};

use {error::*, models::*};

pub mod error;
pub mod models;
pub mod routes;

pub type DynTransactionService = Arc<dyn TransactionService + Send + Sync>;

#[cfg(test)]
mod test;

#[async_trait]
pub trait TransactionService {
    async fn get_transactions(
        &self, 
        user_id: Uuid, 
        get_req: GetTransactionReqModel,
    ) -> Result<Vec<TransactionModel>>;

    async fn create_transactions(
        &self, 
        user_id: Uuid, 
        txs: CreateTransactionsModel
    ) -> Result<()>;

    async fn delete_transactions(
        &self,
        user_id: Uuid, 
        delete_req: DeleteTransactionsModel,
    ) -> Result<()>;
}

pub struct DbConnTransactionService {
    db: DbConn,
    dt_provider: DynDateTimeProvider,
    cc_provider: DynCurrencyConversionProvider,
}

impl DbConnTransactionService {
    pub fn new_dyn(
        db: DbConn, 
        dt_provider: DynDateTimeProvider, 
        cc_provider: DynCurrencyConversionProvider
    ) -> DynTransactionService {
        Arc::new(Self {
            db, dt_provider, cc_provider
        })
    }
}

#[async_trait]
impl TransactionService for DbConnTransactionService {
    async fn get_transactions(
        &self, 
        user_id: Uuid, 
        get_req: GetTransactionReqModel,
    ) -> Result<Vec<TransactionModel>> {
        validate_user_account_role(&self.db, user_id, get_req.account_id, Role::Read).await?;
        let filters = get_req.filters.unwrap_or(vec![]);
        let page_size = get_req.page_size.unwrap_or(15);
        let page_idx = get_req.page_idx.unwrap_or(0);

        // Create the query for the particular account id, and apply each filter
        let mut query = Transactions::find()
            .filter(transactions::Column::AccountId.eq(get_req.account_id));

        for filter in filters {
            query = filter.into_select_query(query);
        }

        // Paginate the results, and fetch the current page
        let pagination = query.paginate(&self.db, page_size);
        let page = pagination.fetch_page(page_idx).await?;

        // Return the transactions in that collection
        Ok(page.into_iter().map(|tx| tx.into()).collect())
    }

    async fn create_transactions(
        &self, 
        user_id: Uuid, 
        create_req: CreateTransactionsModel,
    ) -> Result<()> {
        validate_user_account_role(&self.db, user_id, create_req.account_id, Role::Write).await?;

        // Mapping of category total balance changes
        let mut totals = HashMap::new();
        // All transaction insertions
        let mut insertions = vec![];

        for tx in create_req.txs {
            // Add a new category total, or add to the one already existing
            let am = self.cc_provider.convert(&tx.currency_type, USD_CURRENCY_TYPE, tx.amount).await?;
            *totals.entry(tx.category_id).or_insert(0i64) += am;

            // Create a new transaction to add to the database
            insertions.push(
                transactions::ActiveModel { 
                    account_id:     Set(create_req.account_id), 
                    user_id:        Set(Some(user_id)), 
                    category_id:    Set(Some(tx.category_id)), 
                    timestamp:      Set(self.dt_provider.utc_now()),
                    amount:         Set(am), 
                    notes:          Set(Some(tx.notes)), 
                    is_refill:      Set(false), 

                    ..Default::default()
                }
            );
        }

        let db_tx = self.db.begin().await?;
        Transactions::insert_many(insertions).exec(&db_tx).await?;

        for (cat_id, total) in totals {
            Categories::update_many()
                .filter(categories::Column::Id.eq(cat_id))
                .col_expr(
                    categories::Column::Balance, 
                    Expr::col(categories::Column::Balance).add(total)
                )
                .exec(&db_tx).await?;
        }
        db_tx.commit().await?;

        Ok(())
    }

    async fn delete_transactions(
        &self, 
        user_id: Uuid, 
        delete_req: DeleteTransactionsModel,
    ) -> Result<()> {
        validate_user_account_role(&self.db, user_id, delete_req.account_id, Role::Write).await?;

        // Get all transactions attempting to be deleted
        let txs = Transactions::find().filter(transactions::Column::Id.is_in(delete_req.tx_ids.clone()))
            .all(&self.db).await?;

        // If any transactions do not belong to the particular user's account, return error
        if let Some(tx) = txs.iter().filter(|tx| tx.account_id != delete_req.account_id).next() {
            return Err(Error::AccountDoesNotOwnTransaction(delete_req.account_id, tx.id))?;
        }
        if let Some(tx_id) = delete_req.tx_ids.into_iter()
            .filter(|id| !txs.iter().any(|db_tx| db_tx.id == *id))
            .next() 
        {
            return Err(Error::AccountDoesNotOwnTransaction(delete_req.account_id, tx_id)) ;
        }

        // Get grouped total balance changes for each category
        let mut totals = HashMap::new();
        for tx in &txs {
            *totals.entry(tx.category_id).or_insert(0i64) += tx.amount;
        }

        let tx = self.db.begin().await?;
        for (cat_id, total) in totals {
            Categories::update_many()
                .filter(categories::Column::Id.eq(cat_id))
                .col_expr(
                    categories::Column::Balance, 
                    Expr::col(categories::Column::Balance).sub(total)
                )
                .exec(&tx).await?;
        }

        Transactions::delete_many()
            .filter(transactions::Column::Id.is_in(txs.iter().map(|tx| tx.id)))
            .exec(&tx).await?;
        tx.commit().await?;

        Ok(()) 
    }
}