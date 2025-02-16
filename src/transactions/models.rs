use schmeconomics_entities::transactions;
use sea_orm::{ColumnTrait, prelude::{DateTimeUtc, Uuid}, QueryFilter, Select};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct GetTransactionsQueryParams {
    pub page_size: Option<u64>,
    pub page_idx: Option<u64>,
    pub filters: Option<Vec<TransactionFilter>>,
}

///
/// Model representing a single transaction
/// 
#[derive(Serialize)]
pub struct TransactionModel {
    pub id: i32,
    pub am: i64,
    pub cat_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub timestamp_utc: DateTimeUtc,
    pub notes: Option<String>
}

impl From<transactions::Model> for TransactionModel {
    fn from(value: transactions::Model) -> Self {
        TransactionModel { 
            id: value.id, 
            am: value.amount,
            cat_id: value.category_id,
            user_id: value.user_id, 
            timestamp_utc: value.timestamp, 
            notes: value.notes
        }
    }
}

#[derive(Deserialize)]
pub struct GetTransactionReqModel {
    pub account_id: Uuid,
    pub page_size: Option<u64>, 
    pub page_idx: Option<u64>, 
    pub filters: Option<Vec<TransactionFilter>>
}

pub trait ToSelectQuery {
    fn into_select_query(self, query: Select<transactions::Entity>) -> Select<transactions::Entity>;
}

#[derive(Deserialize)]
pub enum Cmp { Lt, Lte, Eq, Gte, Gt, }

#[derive(Deserialize)]
#[serde(tag = "filter")]
pub enum TransactionFilter {
    CategoryEq { id: Uuid },
    Cmp { cmp: Cmp, val: i64 },
}

impl ToSelectQuery for TransactionFilter {
    fn into_select_query(self, query: Select<transactions::Entity>) -> Select<transactions::Entity> {
        match self {
            TransactionFilter::CategoryEq { id } => query.filter(transactions::Column::CategoryId.eq(id)),
            TransactionFilter::Cmp { cmp, val } => {
                query.filter(
                    match cmp {
                        Cmp::Gt => transactions::Column::Amount.gt(val),
                        Cmp::Gte => transactions::Column::Amount.gte(val),
                        Cmp::Eq => transactions::Column::Amount.eq(val),
                        Cmp::Lt => transactions::Column::Amount.lt(val),
                        Cmp::Lte => transactions::Column::Amount.lte(val),
                    }
                )
            }
        }
    }
}

#[derive(Deserialize)]
pub struct CreateTransactionsModel {
    pub account_id: Uuid,
    pub txs: Vec<CreateTransactionModel>,
}

#[derive(Deserialize)]
pub struct CreateTransactionModel {
    pub category_id: Uuid,
    pub currency_type: String,
    pub amount: i64,
    pub notes: String,
}

#[derive(Deserialize)]
pub struct DeleteTransactionsModel {
    pub account_id: Uuid,
    pub tx_ids: Vec<i32>,
}