use std::env;

use dotenvy::dotenv;
use sqlite::{Connection, Cursor, Row};

#[macro_export]
macro_rules! query {
    // query!(db, "SELECT * FROM categories");
    ( $db:expr, $qry:expr ) => ({
        use $crate::sqlite::RowIter;
        let cursor = $db.prepare($qry).unwrap().into_cursor();
        RowIter(cursor)
    });
    // query!(db, "SELECT * FROM categories WHERE cat_name = ?", Value::String("name"));
    ( $db:expr, $qry:expr, $( $x:expr ),* ) => ({
        use $crate::sqlite::RowIter;
        let cursor =
            $db.prepare($qry).unwrap()
                .into_cursor()
                .bind(&[$($x,)*])
                .unwrap();

        RowIter(cursor)
    });
}

#[macro_export]
macro_rules! execute {
    ( $db:expr, $qry:expr ) => ({
        use $crate::query;
        query!($db, $qry).fold((), |_, _| {});
    });
    ( $db:expr, $qry:expr, $( $x:expr ),* ) => ({
        use $crate::query;
        query!($db, $qry, $( $x ),*).fold((), |_, _| {})
    });
}

///
/// An iterator over a collection of Sqlite rows
///
pub struct RowIter<'a>(pub Cursor<'a>);
impl<'a> Iterator for RowIter<'a> {
    type Item = Row;

    ///
    /// Returns the next Row in the collection,
    /// or None if end of iterator is reached
    ///
    fn next(&mut self) -> Option<Self::Item> {
        // Unwrap the inner Result - only the Option
        // will be saved
        return if let Some(next) = self.0.next() {
            Some(next.unwrap())
        } else {
            None
        };
    }
}

pub fn db() -> Connection {
    dotenv().ok();
    let url = env::var("DATABASE_URL").expect("DATABASE_URL env var is required.");
    sqlite::open(url.clone()).expect(&format!("Could not open db file {}", url))
}
