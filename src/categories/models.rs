use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GetCategoryModel {
    pub id: Uuid,
    pub name: String,
    pub balance: i64,
    pub refill_val: i64,
}

#[derive(Deserialize)]
pub struct CreateCategoryModel {
    pub account_id: Uuid,
    pub name: String,
    pub refill_val: i64,
    pub init_bal: i64,
}

#[derive(Deserialize)]
pub struct UpdateCategoryModel {
    pub account_id: Uuid,
    pub id: Uuid,
    pub new_bal: Option<i64>,
    pub new_name: Option<String>,
    pub new_refill_val: Option<i64>,
}

#[derive(Deserialize)]
pub struct DeleteCategoryModel {
    pub account_id: Uuid,
    pub cat_id: Uuid,
}

#[derive(Deserialize)]
pub struct OrderCategoriesModel {
    pub account_id: Uuid,
    pub orders: Vec<(Uuid, i32)>,
}