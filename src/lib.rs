extern crate hyper;
extern crate mime;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tempdir;
extern crate url;

#[cfg(test)]
#[macro_use]
mod test_macro;

mod client;
mod database;
mod database_name;
mod dbtype;
mod error;
mod transport;

pub mod action;
pub mod testing;

pub use client::{Client, IntoUrl};
pub use database::Database;
pub use database_name::DatabaseName;
pub use dbtype::ErrorResponse;
pub use error::Error;
