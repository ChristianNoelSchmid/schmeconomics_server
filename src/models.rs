use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct GetTransactions {
    pub transacts: Vec<GetTransaction>,
    pub get_complete: bool,
}

#[derive(Serialize)]
pub struct GetTransaction {
    pub user_id: i64,
    pub cat_name: String,
    pub am: i64,
    pub is_refill: bool,
    pub note: Option<String>,
    pub t_stamp: i64,
}

#[derive(Deserialize)]
pub struct PostTransaction {
    pub currency: String, // USD or EUR
    pub cat_id: i64,
    pub am: String,
    pub notes: String,
}

#[derive(Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub cat_name: String,
    pub refill_val: i64,
    pub bal: i64,
}

#[derive(Serialize)]
pub struct User {
    pub id: i64,
    pub user_name: String,
}
