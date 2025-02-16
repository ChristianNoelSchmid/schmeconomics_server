use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("A database error occurred: {0}")]
    DbErr(#[from] sea_orm::DbErr),
    #[error(transparent)]
    DbUtilsErr(#[from] crate::db_utils::DbUtilsError),
}