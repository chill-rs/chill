extern crate hyper;
#[macro_use(mime, __mime__ident_or_ext)]
extern crate mime;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tempdir;
extern crate url;
extern crate uuid;

#[cfg(test)]
#[macro_use]
mod test_macro;

mod client;
mod database;
mod database_name;
mod design_document_name;
mod document;
mod document_id;
mod error;
mod local_document_name;
mod normal_document_name;
mod revision;
mod transport;
mod write_document_response;

pub mod action;
pub mod testing;

pub use client::{Client, IntoUrl};
pub use database::Database;
pub use database_name::DatabaseName;
pub use design_document_name::DesignDocumentName;
pub use document::{Document, DocumentMeta};
pub use document_id::DocumentId;
pub use error::{Error, ErrorResponse};
pub use local_document_name::LocalDocumentName;
pub use normal_document_name::NormalDocumentName;
pub use revision::Revision;
