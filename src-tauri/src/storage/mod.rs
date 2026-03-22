//! SQLite storage engine with FTS5 full-text search.

mod clips;
mod db;
mod error;
mod migrations;
mod pinboards;
mod retention;
mod search;
mod snippets;

pub mod models;

pub use db::Storage;
pub use error::StorageError;
