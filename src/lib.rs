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
mod database;
mod document;
mod error;
mod option;
mod path;
mod revision;
mod serializable_base64_blob;
mod serializable_content_type;
mod serializable_document;
mod transport;
mod write_document_response;

pub mod testing;

pub use attachment::Attachment;
pub use client::{BasicClient, Client, IntoUrl};
pub use database::{BasicDatabase, Database};
pub use document::{BasicDocument, Document};
pub use error::{Error, ErrorResponse};
pub use option::{CreateDatabaseOptions, CreateDocumentOptions, ReadDocumentOptions};
pub use path::{AttachmentName, DatabaseName, DesignDocumentName, DocumentId, LocalDocumentName,
               NormalDocumentName};
pub use revision::Revision;
