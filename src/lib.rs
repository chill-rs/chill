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
extern crate uuid;

pub mod action;
mod client;
mod error;
mod nok_response;
pub mod path;
mod revision;
pub mod testing;
mod transport;

pub use client::{Client, IntoUrl};
pub use error::Error;
pub use nok_response::NokResponse;
pub use path::{AttachmentName, AttachmentPath, DatabaseName, DatabasePath, DesignDocumentName, DesignDocumentPath,
               DocumentId, DocumentPath, IntoAttachmentPath, IntoDatabasePath, IntoDesignDocumentPath,
               IntoDocumentPath, IntoViewPath, LocalDocumentName, NormalDocumentName, ViewName, ViewPath};
pub use revision::Revision;
