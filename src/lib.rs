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

mod client;
mod document;
mod error;
mod revision;
mod transport;

pub mod action;
pub mod path;
pub mod testing;

pub use client::{BasicClient, Client, IntoUrl};
pub use document::{Attachment, Document, SavedAttachment, UnsavedAttachment};
pub use error::{Error, ErrorResponse};
pub use path::{DatabaseName, DatabaseNameBuf, DatabasePath, DatabasePathBuf, DesignDocumentName,
               DesignDocumentNameBuf, DocumentId, DocumentIdBuf, DocumentName, DocumentNameBuf,
               DocumentPath, DocumentPathBuf, IntoDatabasePath, IntoDocumentId, IntoDocumentPath};
pub use revision::Revision;
