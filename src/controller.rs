use std::{collections::HashMap, thread, time::Duration};

use rocket::{get, http::Status, post, routes, serde::json::Json, Build, Rocket};
use sqlite::Value;

use crate::{
    auth::AuthUser,
    execute,
    models::{Category, GetTransaction, GetTransactions, PostTransaction, User},
    query,
    sqlite::db,
};

const TRANSACTS_PER_QUERY: i64 = 25;

pub fn routes(rocket: Rocket<Build>) -> Rocket<Build> {
    let rocket = rocket.mount(
        "/",
        routes![
            get_users,
            get_categories,
            post_adjust_refills,
            get_transactions,
            post_transaction,
            post_refill
        ],
    );
    rocket
}

#[get("/users")]
fn get_users(_user: AuthUser) -> Json<Vec<User>> {
    let db = db();
    return Json(
        query!(db, "SELECT * from users")
            .map(|row| User {
                id: row.get("id"),
                user_name: row.get("user_name"),
            })
            .collect(),
    );
}

#[post("/adjust-refills", format = "json", data = "<cats>")]
fn post_adjust_refills(cats: Json<HashMap<i64, i64>>, _user: AuthUser) -> Status {
    let db = db();
    for (cat_id, refill) in cats.0 {
        execute!(
            db,
            "UPDATE categories SET refill_val = ? WHERE id = ?",
            Value::Integer(refill),
            Value::Integer(cat_id)
        );
    }
    Status::Ok
}

#[post("/refill")]
fn post_refill(user: AuthUser) -> Status {
    let db = db();
    let mut iter = query!(db, "SELECT id, refill_val FROM categories")
        .map(|row| {
            (
                row.get::<i64, &str>("id"),
                row.get::<i64, &str>("refill_val"),
            )
        })
        .peekable();

    let mut transact_cmd =
        String::from("INSERT INTO transactions (cat_id, user_id, am, is_refill) VALUES ");
    while let Some((id, refill_val)) = iter.next() {
        transact_cmd.push_str(&format!("({}, {}, {}, true)", id, user.0, refill_val));
        if let Some(_) = iter.peek() {
            transact_cmd.push_str(", ");
        } else {
            transact_cmd.push_str("; ");
        }
    }

    execute!(db, &transact_cmd);
    execute!(db, "UPDATE categories SET bal = bal + refill_val;");
    Status::Ok
}

#[get("/categories")]
fn get_categories(_user: AuthUser) -> Json<Vec<Category>> {
    let db = db();
    let rows = query!(db, "SELECT * FROM categories ORDER BY id;");

    return Json(
        rows.map(|row| Category {
            id: row.get("id"),
            cat_name: row.get("cat_name"),
            refill_val: row.get("refill_val"),
            bal: row.get("bal"),
        })
        .collect(),
    );
}

#[get("/transactions/<cat_id>/<offset_group>")]
fn get_transactions(_user: AuthUser, cat_id: i64, offset_group: i64) -> Json<GetTransactions> {
    let db = db();

    // Filter the query by the cateory id if
    // one is given (ie. if it isn't -1). Otherwise
    // search all categories.
    let cat_filter = if cat_id != -1 {
        format!("\nWHERE cat_id = {}\n", cat_id)
    } else {
        String::new()
    };

    let rows = query!(
        db,
        &format!(
            r#"
            SELECT user_id, cat_name, am, is_refill, notes, t_stamp 
            FROM transactions t JOIN categories c ON c.id=t.cat_id{cat_filter} 
            ORDER BY t_stamp DESC
            LIMIT ? OFFSET ?
        "#
        ),
        Value::Integer(TRANSACTS_PER_QUERY),
        Value::Integer(offset_group * TRANSACTS_PER_QUERY)
    );
    let count = query!(
        db,
        &format!(
            r#"
            SELECT COUNT(*) as count 
            FROM transactions t JOIN categories c ON c.id=t.cat_id{cat_filter} 
        "#
        )
    )
    .next()
    .unwrap()
    .get::<i64, &str>("count");

    Json(GetTransactions {
        transacts: rows
            .map(|row| GetTransaction {
                user_id: row.get("user_id"),
                cat_name: row.get("cat_name"),
                am: row.get("am"),
                is_refill: row.get::<i64, &str>("is_refill") == 1,
                note: row.try_get("notes").ok(),
                t_stamp: row.get("t_stamp"),
            })
            .collect(),
        get_complete: ((offset_group) * TRANSACTS_PER_QUERY + TRANSACTS_PER_QUERY) >= count,
    })
}

#[post("/transaction", format = "application/json", data = "<transact>")]
fn post_transaction(transact: Json<PostTransaction>, user: AuthUser) -> Status {
    let am = transact.am.clone();
    let am = am.replace(".", "");
    let am = am.parse::<i64>().unwrap();
    return {
        let db = db();
        execute!(
            db,
            "UPDATE categories SET bal = bal + ? WHERE id = ?;",
            Value::Integer(am),
            Value::Integer(transact.cat_id)
        );
        execute!(
            db,
            r#" INSERT INTO transactions (cat_id, user_id, am, notes) VALUES (?, ?, ?, ?);"#,
            Value::Integer(transact.cat_id),
            Value::Integer(user.0),
            Value::Integer(am),
            Value::String(transact.notes.clone())
        );
        Status::Created
    };
}
