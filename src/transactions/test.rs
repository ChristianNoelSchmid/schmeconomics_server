use std::sync::Arc;

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use mockall::predicate::{always, eq};
use sea_orm::{prelude::Uuid, sea_query::TableCreateStatement, ConnectionTrait, Database, DbBackend, DbConn, EntityTrait, Schema, Set};

use schmeconomics_entities::{account_users, accounts, categories, prelude::*, users};
use utils_rs::date_time_provider::MockDateTimeProvider;

use crate::{currency_conv_provider::{MockCurrencyConversionProvider, USD_CURRENCY_TYPE}, db_utils::DbUtilsError, transactions::{models::{DeleteTransactionsModel, GetTransactionReqModel}, CreateTransactionModel, Error, TransactionService}};

use super::{models::CreateTransactionsModel, DbConnTransactionService, TransactionFilter};

lazy_static! {
    static ref TEST_USER_1_ID: Uuid = Uuid::parse_str("be5ca263-2307-4e5a-acbd-3281fb81ea60").unwrap();
    static ref TEST_USER_2_ID: Uuid = Uuid::parse_str("e8411903-c326-4ffe-9dd0-cb766b9299e4").unwrap();
    static ref TEST_ACCOUNT_1_ID: Uuid = Uuid::parse_str("f017369e-9dd1-4434-b197-40361cc0dbcd").unwrap();
    static ref TEST_ACCOUNT_2_ID: Uuid = Uuid::parse_str("2dd9ffbe-5d15-401f-b637-7f3e2de9bf1f").unwrap();
    static ref TEST_CAT_1_ID: Uuid = Uuid::parse_str("c8be0f8e-629e-46ce-9e76-e691caa0714b").unwrap();
    static ref TEST_CAT_2_ID: Uuid = Uuid::parse_str("0fd2a2ce-cce1-43c4-a69d-8b1b523f0127").unwrap();

    static ref TEST_CAT_1_ORIG_BAL: i64 = 1000;
    static ref TEST_CAT_2_ORIG_BAL: i64 = 14000;

    // 2024-11-10 12:03:34
    static ref TEST_DT: DateTime<Utc> = DateTime::<Utc>::from_timestamp_millis(1731240214000).unwrap();
}

async fn create_test_db() -> anyhow::Result<DbConn> {
    // In-memory Sqlite connection
    let db = Database::connect("sqlite::memory:").await?;

    // Schema and Tables SeaOrm statements
    let schema = Schema::new(DbBackend::Sqlite);
    let user_stmt: TableCreateStatement = schema.create_table_from_entity(Users);
    let account_stmt: TableCreateStatement = schema.create_table_from_entity(Accounts);
    let account_user_stmt: TableCreateStatement = schema.create_table_from_entity(AccountUsers);
    let category_stmt: TableCreateStatement = schema.create_table_from_entity(Categories);
    let tx_stmt: TableCreateStatement = schema.create_table_from_entity(Transactions);

    db.execute(db.get_database_backend().build(&user_stmt)).await?;
    db.execute(db.get_database_backend().build(&account_stmt)).await?;
    db.execute(db.get_database_backend().build(&account_user_stmt)).await?;
    db.execute(db.get_database_backend().build(&category_stmt)).await?;
    db.execute(db.get_database_backend().build(&tx_stmt)).await?;

    // Insert 1st test user
    let new_user = users::ActiveModel {
        id: Set(*TEST_USER_1_ID),
        email: Set(String::from("user1@mail.com")),
        email_verified: Set(true),
        password_hash: Set(String::from("password")),
        name: Set(String::from("tester 1")),

        ..Default::default()
    };
    // model_fn(&mut new_user);
    Users::insert(new_user).exec(&db).await?;

    // Create 1st test account
    let account = accounts::ActiveModel {
        id: Set(*TEST_ACCOUNT_1_ID),
    };
    Accounts::insert(account).exec(&db).await?;

    let account_user = account_users::ActiveModel {
        account_id: Set(*TEST_ACCOUNT_1_ID),
        user_id: Set(*TEST_USER_1_ID),
    };
    AccountUsers::insert(account_user).exec(&db).await?;

    // Insert 2nd test user
    let new_user = users::ActiveModel {
        id: Set(*TEST_USER_2_ID),
        email: Set(String::from("user2@mail.com")),
        email_verified: Set(true),
        password_hash: Set(String::from("password")),
        name: Set(String::from("tester 2")),

        ..Default::default()
    };
    Users::insert(new_user).exec(&db).await?;

    // Create 2nd test account
    let account = accounts::ActiveModel {
        id: Set(*TEST_ACCOUNT_2_ID),
    };
    Accounts::insert(account).exec(&db).await?;

    // Insert test categories
    let cat1 = categories::ActiveModel {
        id: Set(*TEST_CAT_1_ID),
        account_id: Set(*TEST_ACCOUNT_1_ID),
        name: Set(String::from("Cat1")),
        balance: Set(*TEST_CAT_1_ORIG_BAL),
        refill_value: Set(0),
        order: Set(1),
    };
    let cat2 = categories::ActiveModel {
        id: Set(*TEST_CAT_2_ID),
        account_id: Set(*TEST_ACCOUNT_1_ID),
        name: Set(String::from("Cat2")),
        balance: Set(*TEST_CAT_2_ORIG_BAL),
        refill_value: Set(0),
        order: Set(2),
    };
    Categories::insert_many(vec![cat1, cat2]).exec(&db).await?;
 
    Ok(db)
}

async fn create_test_service() -> anyhow::Result<(DbConnTransactionService, DbConn)> {
    let db = create_test_db().await?;

    // DateTimeProvider
    let mut mock_dt_service = MockDateTimeProvider::new();
    mock_dt_service.expect_utc_now().returning(|| TEST_DT.clone());
    let mock_dt_service = Arc::new(mock_dt_service);

    // CurrencyConversionProvider
    // Conversion taken from Feb. 3rd, 2025 (rounded to nearest half for testing)
    let mut mock_cc_provider = MockCurrencyConversionProvider::new();
    mock_cc_provider.expect_convert()
        .with(eq(USD_CURRENCY_TYPE), eq("CAD"), always())
        .returning(|_, _, am| Ok((am as f64 * 2.0).floor() as i64));
    mock_cc_provider.expect_convert()
        .with(eq("CAD"), eq(USD_CURRENCY_TYPE), always())
        .returning(|_, _, am| Ok((am as f64 * 0.5).floor() as i64));
    mock_cc_provider.expect_convert()
        .with(eq(USD_CURRENCY_TYPE), eq(USD_CURRENCY_TYPE), always())
        .returning(|_, _, am| Ok(am));

    let mock_cc_provider = Arc::new(mock_cc_provider);

    // Service
    let svc = DbConnTransactionService {
        db: db.clone(),
        dt_provider: mock_dt_service,
        cc_provider: mock_cc_provider,
    };

    Ok((svc, db))
}

async fn test_transact_1(svc: &DbConnTransactionService) -> anyhow::Result<()> {
    svc.create_transactions(
        *TEST_USER_1_ID, 
        CreateTransactionsModel {
            account_id: *TEST_ACCOUNT_1_ID, 
            txs: vec![
                CreateTransactionModel { 
                    category_id: TEST_CAT_1_ID.clone(),
                    amount: 1000, 
                    notes: String::from("Notes1"),
                    currency_type: USD_CURRENCY_TYPE.to_string()
                }
            ]
        }
    ).await?;

    Ok(())
}

async fn test_transact_2(svc: &DbConnTransactionService) -> anyhow::Result<()> {
    svc.create_transactions(
        *TEST_USER_1_ID, 
        CreateTransactionsModel { 
            account_id: *TEST_ACCOUNT_1_ID, 
            txs: vec![
                CreateTransactionModel { 
                    category_id: *TEST_CAT_1_ID,
                    amount: 3000, 
                    notes: String::from("Notes2"),
                    currency_type: USD_CURRENCY_TYPE.to_string(),
                },
                CreateTransactionModel { 
                    category_id: *TEST_CAT_1_ID,
                    amount: 5000, 
                    notes: String::from("Notes3"),
                    currency_type: USD_CURRENCY_TYPE.to_string(),
                },
                CreateTransactionModel { 
                    category_id: *TEST_CAT_1_ID,
                    amount: -1500, 
                    notes: String::from("Notes4"),
                    currency_type: USD_CURRENCY_TYPE.to_string(),
                },
                CreateTransactionModel { 
                    category_id: *TEST_CAT_2_ID,
                    amount: -300, 
                    notes: String::from("Notes5"),
                    currency_type: USD_CURRENCY_TYPE.to_string(),
                }
            ]
        }
    ).await?;

    Ok(())
}

#[tokio::test]
async fn test_new_transaction() -> anyhow::Result<()> {
    let (svc, db) = create_test_service().await?;

    // 1st Transaction
    test_transact_1(&svc).await?;
    let tx = Transactions::find_by_id(1).one(&db).await?;
    assert!(tx.is_some());

    let tx = tx.unwrap();
    assert_eq!(1, tx.id);
    assert_eq!(*TEST_ACCOUNT_1_ID, tx.account_id);
    assert_eq!(Some(*TEST_USER_1_ID), tx.user_id);
    assert_eq!(1000, tx.amount);
    assert_eq!(false, tx.is_refill);
    assert_eq!(*TEST_DT, tx.timestamp);
    assert_eq!("Notes1", tx.notes.unwrap());

    let cats = Categories::find().all(&db).await?;
    assert_eq!(*TEST_CAT_1_ORIG_BAL + 1000, cats[0].balance);
    assert_eq!(*TEST_CAT_2_ORIG_BAL, cats[1].balance);

    // 2nd Transaction
    test_transact_2(&svc).await?;
    let txs = Transactions::find().all(&db).await?;
    assert_eq!(5, txs.len());

    assert_eq!(2, txs[1].id);
    assert_eq!(*TEST_ACCOUNT_1_ID, txs[1].account_id);
    assert_eq!(Some(*TEST_USER_1_ID), txs[1].user_id);
    assert_eq!(Some(*TEST_CAT_1_ID), txs[1].category_id);
    assert_eq!(3000, txs[1].amount);
    assert_eq!(false, txs[1].is_refill);
    assert_eq!(*TEST_DT, txs[1].timestamp);
    assert_eq!(Some(String::from("Notes2")), txs[1].notes);

    assert_eq!(3, txs[2].id);
    assert_eq!(*TEST_ACCOUNT_1_ID, txs[2].account_id);
    assert_eq!(Some(*TEST_USER_1_ID), txs[2].user_id);
    assert_eq!(Some(*TEST_CAT_1_ID), txs[2].category_id);
    assert_eq!(5000, txs[2].amount);
    assert_eq!(false, txs[2].is_refill);
    assert_eq!(*TEST_DT, txs[2].timestamp);
    assert_eq!(Some(String::from("Notes3")), txs[2].notes);

    assert_eq!(4, txs[3].id);
    assert_eq!(*TEST_ACCOUNT_1_ID, txs[3].account_id);
    assert_eq!(Some(*TEST_USER_1_ID), txs[3].user_id);
    assert_eq!(Some(*TEST_CAT_1_ID), txs[3].category_id);
    assert_eq!(-1500, txs[3].amount);
    assert_eq!(false, txs[3].is_refill);
    assert_eq!(*TEST_DT, txs[3].timestamp);
    assert_eq!(Some(String::from("Notes4")), txs[3].notes);

    assert_eq!(5, txs[4].id);
    assert_eq!(*TEST_ACCOUNT_1_ID, txs[4].account_id);
    assert_eq!(Some(*TEST_USER_1_ID), txs[4].user_id);
    assert_eq!(Some(*TEST_CAT_2_ID), txs[4].category_id);
    assert_eq!(-300, txs[4].amount);
    assert_eq!(false, txs[4].is_refill);
    assert_eq!(*TEST_DT, txs[4].timestamp);
    assert_eq!(Some(String::from("Notes5")), txs[4].notes);

    let cats = Categories::find().all(&db).await?;
    assert_eq!(*TEST_CAT_1_ORIG_BAL + 1000 + 3000 + 5000 - 1500, cats[0].balance);
    assert_eq!(*TEST_CAT_2_ORIG_BAL - 300, cats[1].balance);

    Ok(())
}

#[tokio::test]
async fn test_cc_txs() -> anyhow::Result<()> {
    let (svc, db) = create_test_service().await?;

    // 1st Transaction
    svc.create_transactions(
        *TEST_USER_1_ID, 
        CreateTransactionsModel {
            account_id: *TEST_ACCOUNT_1_ID, 
            txs: vec![
                CreateTransactionModel { 
                    category_id: *TEST_CAT_1_ID, 
                    currency_type: "CAD".to_string(), 
                    amount: 1000, 
                    notes: String::new(),
                }
            ]
        }
    ).await?;
    let tx = Transactions::find_by_id(1).one(&db).await?;
    assert!(tx.is_some());

    let tx = tx.unwrap();
    assert_eq!(1, tx.id);
    assert_eq!(500, tx.amount);

    let cats = Categories::find().all(&db).await?;
    assert_eq!(*TEST_CAT_1_ORIG_BAL + 500, cats[0].balance);

    Ok(())
}

#[tokio::test]
async fn test_delete_transactions() -> anyhow::Result<()> {
    let (svc, db) = create_test_service().await?;
    test_transact_1(&svc).await?;
    test_transact_2(&svc).await?;

    svc.delete_transactions(
        *TEST_USER_1_ID, 
        DeleteTransactionsModel { 
            account_id: *TEST_ACCOUNT_1_ID, 
            tx_ids: vec![1, 3, 5]
        }
    ).await?;

    let txs = Transactions::find().all(&db).await?;
    assert_eq!(2, txs.len());

    assert_eq!(2, txs[0].id);
    assert_eq!(*TEST_ACCOUNT_1_ID, txs[0].account_id);
    assert_eq!(Some(*TEST_USER_1_ID), txs[0].user_id);
    assert_eq!(Some(*TEST_CAT_1_ID), txs[0].category_id);
    assert_eq!(3000, txs[0].amount);
    assert_eq!(false, txs[0].is_refill);
    assert_eq!(*TEST_DT, txs[0].timestamp);
    assert_eq!(Some(String::from("Notes2")), txs[0].notes);

    assert_eq!(4, txs[1].id);
    assert_eq!(*TEST_ACCOUNT_1_ID, txs[1].account_id);
    assert_eq!(Some(*TEST_USER_1_ID), txs[1].user_id);
    assert_eq!(Some(*TEST_CAT_1_ID), txs[1].category_id);
    assert_eq!(-1500, txs[1].amount);
    assert_eq!(false, txs[1].is_refill);
    assert_eq!(*TEST_DT, txs[1].timestamp);
    assert_eq!(Some(String::from("Notes4")), txs[1].notes);

    let cats = Categories::find().all(&db).await?;
    assert_eq!(*TEST_CAT_1_ORIG_BAL + 3000 - 1500, cats[0].balance);
    assert_eq!(*TEST_CAT_2_ORIG_BAL, cats[1].balance);

    Ok(())
}

#[tokio::test]
async fn test_attempt_delete_user_does_not_own_account() -> anyhow::Result<()> {
    let (svc, _db) = create_test_service().await?;
    test_transact_1(&svc).await?;
    test_transact_2(&svc).await?;

    let res = svc.delete_transactions(
        *TEST_USER_2_ID, 
        DeleteTransactionsModel { 
            account_id: *TEST_ACCOUNT_1_ID, 
            tx_ids: vec![1, 2, 3, 4, 5] 
        }
    ).await;

    assert!(
        matches!(
            res, 
            Err(Error::DbUtilsError(DbUtilsError::UserNotPartOfAccount(user_id, account_id)))
                if user_id == *TEST_USER_2_ID && account_id == *TEST_ACCOUNT_1_ID
        )
    );

    Ok(())
}

#[tokio::test]
async fn test_attempt_delete_account_does_not_own_transaction() -> anyhow::Result<()> {
    let (svc, _db) = create_test_service().await?;
    test_transact_1(&svc).await?;
    test_transact_2(&svc).await?;

    let res = svc.delete_transactions(
        *TEST_USER_1_ID, 
        DeleteTransactionsModel { 
            account_id: *TEST_ACCOUNT_1_ID, 
            tx_ids: vec![4, 18] 
        }
    ).await;

    assert!(
        matches!(
            res, 
            Err(Error::AccountDoesNotOwnTransaction(account_id, tx_id))
                if account_id == *TEST_ACCOUNT_1_ID && tx_id == 18
        )
    );

    Ok(())
}

#[tokio::test]
async fn test_get_txs_with_cat_filter() -> anyhow::Result<()> {
    let (svc, _db) = create_test_service().await?;
    test_transact_1(&svc).await?;
    test_transact_2(&svc).await?;

    let cat_1_txs = svc.get_transactions(
        *TEST_USER_1_ID,
        GetTransactionReqModel {
            account_id: *TEST_ACCOUNT_1_ID,
            page_size: Some(25), 
            page_idx: Some(0), 
            filters: Some(vec![TransactionFilter::CategoryEq { id: *TEST_CAT_1_ID }]),
        }
    ).await?;

    assert_eq!(4, cat_1_txs.len());
    assert_eq!(1, cat_1_txs[0].id);
    assert_eq!(*TEST_DT, cat_1_txs[0].timestamp_utc);
    assert_eq!(Some(*TEST_USER_1_ID), cat_1_txs[0].user_id);
    assert_eq!(Some(String::from("Notes1")), cat_1_txs[0].notes);

    assert_eq!(2, cat_1_txs[1].id);
    assert_eq!(*TEST_DT, cat_1_txs[1].timestamp_utc);
    assert_eq!(Some(*TEST_USER_1_ID), cat_1_txs[1].user_id);
    assert_eq!(Some(String::from("Notes2")), cat_1_txs[1].notes);

    assert_eq!(3, cat_1_txs[2].id);
    assert_eq!(*TEST_DT, cat_1_txs[2].timestamp_utc);
    assert_eq!(Some(*TEST_USER_1_ID), cat_1_txs[2].user_id);
    assert_eq!(Some(String::from("Notes3")), cat_1_txs[2].notes);

    assert_eq!(4, cat_1_txs[3].id);
    assert_eq!(*TEST_DT, cat_1_txs[3].timestamp_utc);
    assert_eq!(Some(*TEST_USER_1_ID), cat_1_txs[3].user_id);
    assert_eq!(Some(String::from("Notes4")), cat_1_txs[3].notes);

    let cat_2_txs = svc.get_transactions(
        *TEST_USER_1_ID,
        GetTransactionReqModel { 
            account_id: *TEST_ACCOUNT_1_ID,
            page_size: Some(25), 
            page_idx: Some(0), 
            filters: Some(vec![TransactionFilter::CategoryEq { id: *TEST_CAT_2_ID }]) 
        }
    ).await?;

    assert_eq!(4, cat_1_txs.len());
    assert_eq!(5, cat_2_txs[0].id);
    assert_eq!(*TEST_DT, cat_2_txs[0].timestamp_utc);
    assert_eq!(Some(*TEST_USER_1_ID), cat_2_txs[0].user_id);
    assert_eq!(Some(String::from("Notes5")), cat_2_txs[0].notes);

    Ok(())
}