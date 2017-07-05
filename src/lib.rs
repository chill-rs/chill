//! Chill is a CouchDB client-side library.

extern crate futures;
extern crate regex;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tempdir;
extern crate url;

mod action;
mod client;
mod error;
mod nok_response;
pub mod testing;
mod transport;

pub use client::{Client, IntoUrl};
pub use error::Error;
pub use nok_response::NokResponse;
