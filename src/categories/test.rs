use chrono::Utc;
use lazy_static::lazy_static;
use sea_orm::{sea_query::TableCreateStatement, ConnectionTrait, Database, DbBackend, DbConn, EntityTrait, Schema, Set};
use uuid::Uuid;

use schmeconomics_entities::{account_users, accounts, categories, prelude::*, users};

use crate::{categories::{models::DeleteCategoryModel, CategoryService, CreateCategoryModel, Error, UpdateCategoryModel}, db_utils::Role};

use super::DbConnCategoryService;

lazy_static! {
    static ref TEST_USER_1_ID: Uuid = Uuid::parse_str("be5ca263-2307-4e5a-acbd-3281fb81ea60").unwrap();
    static ref TEST_USER_2_ID: Uuid = Uuid::parse_str("e8411903-c326-4ffe-9dd0-cb766b9299e4").unwrap();
    static ref TEST_ACCOUNT_1_ID: Uuid = Uuid::parse_str("f017369e-9dd1-4434-b197-40361cc0dbcd").unwrap();
    static ref TEST_ACCOUNT_2_ID: Uuid = Uuid::parse_str("2dd9ffbe-5d15-401f-b637-7f3e2de9bf1f").unwrap();
    static ref TEST_CAT_1_ID: Uuid = Uuid::parse_str("c8be0f8e-629e-46ce-9e76-e691caa0714b").unwrap();
    static ref TEST_CAT_2_ID: Uuid = Uuid::parse_str("0fd2a2ce-cce1-43c4-a69d-8b1b523f0127").unwrap();

    static ref TEST_CAT_1_ORIG_BAL: i64 = 1000;
    static ref TEST_CAT_2_ORIG_BAL: i64 = 14000;
}

async fn create_test_db(create_cats: bool) -> anyhow::Result<DbConn> {
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
        created_on_utc: Set(Utc::now()),
        two_factor_enabled: Set(false),

        ..Default::default()
    };
    // model_fn(&mut new_user);
    Users::insert(new_user).exec(&db).await?;

    // Create 1st test account
    let account = accounts::ActiveModel {
        id: Set(*TEST_ACCOUNT_1_ID),
        ..Default::default()
    };
    Accounts::insert(account).exec(&db).await?;

    // Insert 2nd test user
    let new_user = users::ActiveModel {
        id: Set(*TEST_USER_2_ID),
        email: Set(String::from("user2@mail.com")),
        email_verified: Set(true),
        password_hash: Set(String::from("password")),
        name: Set(String::from("tester 2")),
        created_on_utc: Set(Utc::now()),
        two_factor_enabled: Set(false),

        ..Default::default()
    };
    Users::insert(new_user).exec(&db).await?;

    // Create 2nd test account
    let account = accounts::ActiveModel {
        id: Set(*TEST_ACCOUNT_2_ID),
        ..Default::default()
    };
    Accounts::insert(account).exec(&db).await?;

    let account_user = account_users::ActiveModel {
        account_id: Set(*TEST_ACCOUNT_1_ID),
        user_id: Set(*TEST_USER_1_ID),
        role: Set(Role::Admin.to_string()),
        verified: Set(true),
        created_on: Set(Utc::now()),
    };
    AccountUsers::insert(account_user).exec(&db).await?;


    // Insert test categories if create_cats is true
    if create_cats {
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
    }
 
    Ok(db)
}

async fn create_test_service(create_cats: bool) -> anyhow::Result<(DbConnCategoryService, DbConn)> {
    let db = create_test_db(create_cats).await?;

    // Service
    let svc = DbConnCategoryService {
        db: db.clone(),
    };

    Ok((svc, db))
}

#[tokio::test]
async fn test_order_assignment_with_no_categories() -> anyhow::Result<()> {
    let (svc, db) = create_test_service(false).await?;
    let new_cat = svc.create_cat(*TEST_USER_1_ID, CreateCategoryModel { 
        account_id: *TEST_ACCOUNT_1_ID,
        name: String::from("Cat1"), 
        refill_val: 2000, 
        init_bal: 1000, 
    }).await?;

    let db_cat = Categories::find_by_id(new_cat.id).one(&db).await?;
    assert!(db_cat.is_some());

    let db_cat = db_cat.unwrap();
    assert_eq!(*TEST_ACCOUNT_1_ID, db_cat.account_id);
    assert_eq!(1000, db_cat.balance);
    assert_eq!(2000, db_cat.refill_value);
    assert_eq!(1, db_cat.order);
    assert_eq!("Cat1", db_cat.name);

    assert_eq!(db_cat.name, new_cat.name);
    assert_eq!(db_cat.refill_value, new_cat.refill_val);
    assert_eq!(db_cat.balance, new_cat.balance);

    Ok(())
}

#[tokio::test]
async fn test_creation_of_category() -> anyhow::Result<()> {
    let (svc, db) = create_test_service(true).await?;
    let new_cat = svc.create_cat(
        *TEST_USER_1_ID, 
            CreateCategoryModel { 
            account_id: *TEST_ACCOUNT_1_ID,
            name: String::from("Cat3"), 
            refill_val: 2000, 
            init_bal: 1000, 
        }
    ).await?;

    let db_cat = Categories::find_by_id(new_cat.id).one(&db).await?;
    assert!(db_cat.is_some());

    let db_cat = db_cat.unwrap();
    assert_eq!(*TEST_ACCOUNT_1_ID, db_cat.account_id);
    assert_eq!(1000, db_cat.balance);
    assert_eq!(2000, db_cat.refill_value);
    assert_eq!(3, db_cat.order);
    assert_eq!("Cat3", db_cat.name);

    assert_eq!(db_cat.name, new_cat.name);
    assert_eq!(db_cat.refill_value, new_cat.refill_val);
    assert_eq!(db_cat.balance, new_cat.balance);

    Ok(())
}

#[tokio::test]
async fn test_update_category() -> anyhow::Result<()> {
    let (svc, db) = create_test_service(true).await?;

    let update_cat_1 = svc.update_cat(
        *TEST_USER_1_ID,
         UpdateCategoryModel { 
            account_id: *TEST_ACCOUNT_1_ID, 
            id: *TEST_CAT_1_ID,
            new_name: Some(String::from("NewCat1")), 
            new_refill_val: None,
            new_bal: None,
        }
    ).await?;

    let update_cat_2 = svc.update_cat(
        *TEST_USER_1_ID,
        UpdateCategoryModel { 
            account_id: *TEST_ACCOUNT_1_ID, 
            id: *TEST_CAT_2_ID,
            new_name: None, 
            new_refill_val: Some(1200),
            new_bal: Some(200)
        }
    ).await?;

    let cats = Categories::find().all(&db).await?;

    assert_eq!(*TEST_ACCOUNT_1_ID, cats[0].account_id);
    assert_eq!(1000, cats[0].balance);
    assert_eq!(0, cats[0].refill_value);
    assert_eq!("NewCat1", cats[0].name);

    assert_eq!(cats[0].id, update_cat_1.id);
    assert_eq!(cats[0].name, update_cat_1.name);
    assert_eq!(cats[0].refill_value, update_cat_1.refill_val);
    assert_eq!(cats[0].balance, update_cat_1.balance);

    assert_eq!(*TEST_ACCOUNT_1_ID, cats[1].account_id);
    assert_eq!(200, cats[1].balance);
    assert_eq!(1200, cats[1].refill_value);
    assert_eq!("Cat2", cats[1].name);

    assert_eq!(cats[1].id, update_cat_2.id);
    assert_eq!(cats[1].name, update_cat_2.name);
    assert_eq!(cats[1].refill_value, update_cat_2.refill_val);
    assert_eq!(cats[1].balance, update_cat_2.balance);

    Ok(())
}

#[tokio::test]
async fn test_update_cat_does_not_exist() -> anyhow::Result<()> {
    let (svc, _db) = create_test_service(true).await?;
    let test_id = uuid::Uuid::now_v7();
    let res = svc.update_cat(
        *TEST_USER_1_ID,
        UpdateCategoryModel { 
            account_id: *TEST_ACCOUNT_1_ID,
            id: test_id, 
            new_name: Some(String::from("NewCat1")), 
            new_refill_val: Some(1000),
            new_bal: None,
        }
    ).await;

    assert!(matches!(res, Err(Error::CategoryNotFound(id)) if id == test_id));
    let non_ex_cat_id = Uuid::now_v7();

    // Category exists, but provided account doesn't own it
    let res = svc.update_cat(
        *TEST_USER_1_ID,
         UpdateCategoryModel { 
            account_id: *TEST_ACCOUNT_1_ID,
            id: non_ex_cat_id,
            new_name: Some(String::from("NewCat1")), 
            new_refill_val: Some(1000),
            new_bal: None
        }
    ).await;

    assert!(matches!(res, Err(Error::CategoryNotFound(id)) if id == non_ex_cat_id));

    Ok(())
}

#[tokio::test]
async fn test_update_cat_name_reused() -> anyhow::Result<()> {
    let (svc, _db) = create_test_service(true).await?;
    let res = svc.update_cat(
        *TEST_USER_1_ID,
        UpdateCategoryModel { 
            account_id: *TEST_ACCOUNT_1_ID,
            id: *TEST_CAT_1_ID, 
            new_name: Some(String::from("Cat2")), 
            new_refill_val: None,
            new_bal: None,
        }
    ).await;

    assert!(matches!(res, Err(Error::NameReuse(name)) if name == String::from("Cat2")));

    let res = svc.update_cat(
        *TEST_USER_1_ID, 
        UpdateCategoryModel { 
            account_id: *TEST_ACCOUNT_1_ID,
            id: *TEST_CAT_1_ID, 
            new_name: Some(String::from("  cAT2 ")), 
            new_refill_val: None,
            new_bal: None,
        }
    ).await;

    assert!(matches!(res, Err(Error::NameReuse(name)) if name == String::from("cAT2")));

    Ok(())
}

#[tokio::test]
async fn test_create_cat_name_reused() -> anyhow::Result<()> {
    let (svc, _db) = create_test_service(true).await?;
    let res = svc.create_cat(
        *TEST_USER_1_ID, 
        CreateCategoryModel { 
            account_id: *TEST_ACCOUNT_1_ID, 
            name: String::from("Cat1"), 
            refill_val: 1000, 
            init_bal: 1000 
        }
    ).await;

    assert!(matches!(res, Err(Error::NameReuse(name)) if name == String::from("Cat1")));

    let res = svc.create_cat(
        *TEST_USER_1_ID,
        CreateCategoryModel { 
            account_id: *TEST_ACCOUNT_1_ID, 
            name: String::from("\t  caT1  \t"), 
            refill_val: 1000,
            init_bal: 1000,
        }
    ).await;

    assert!(matches!(res, Err(Error::NameReuse(name)) if name == String::from("caT1")));

    Ok(())

}

#[tokio::test]
async fn test_delete_cat_success() -> anyhow::Result<()> {
    let (svc, db) = create_test_service(true).await?;
    svc.delete_cat(
        *TEST_USER_1_ID, 
        DeleteCategoryModel {
            account_id: *TEST_ACCOUNT_1_ID, 
            cat_id: *TEST_CAT_1_ID
        }
    ).await?;

    let cats = Categories::find().all(&db).await?;
    assert_eq!(1, cats.len());

    assert_eq!(*TEST_CAT_2_ID, cats[0].id);

    Ok(())
}

#[tokio::test]
async fn test_delete_cat_does_not_exist_err() -> anyhow::Result<()> {
    let (svc, _db) = create_test_service(true).await?;
    let test_id = Uuid::now_v7();
    let res = svc.delete_cat(
        *TEST_USER_1_ID,
        DeleteCategoryModel {
            account_id: *TEST_ACCOUNT_1_ID, 
            cat_id: test_id
        }
    ).await;

    assert!(matches!(res, Err(Error::CategoryNotFound(id)) if id == test_id));

    Ok(())
}