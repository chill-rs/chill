extern crate base64;
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

mod attachment;
mod client;
mod create_database_options;
mod create_document_options;
mod database;
mod database_name;
mod design_document_name;
mod document;
mod document_id;
mod error;
mod local_document_name;
mod normal_document_name;
mod read_document_options;
mod revision;
mod serializable_base64_blob;
mod serializable_content_type;
mod serializable_document;
mod transport;
mod write_document_response;

pub mod testing;

pub use attachment::Attachment;
pub use client::{BasicClient, Client, IntoUrl};
pub use create_database_options::CreateDatabaseOptions;
pub use create_document_options::CreateDocumentOptions;
pub use database::{BasicDatabase, Database};
pub use database_name::DatabaseName;
pub use design_document_name::DesignDocumentName;
pub use document::{BasicDocument, Document};
pub use document_id::DocumentId;
pub use error::{Error, ErrorResponse};
pub use local_document_name::LocalDocumentName;
pub use normal_document_name::NormalDocumentName;
pub use read_document_options::ReadDocumentOptions;
pub use revision::Revision;
