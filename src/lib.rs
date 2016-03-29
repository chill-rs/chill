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
mod view;

pub mod action;
pub mod path;
pub mod testing;

pub use client::{Client, IntoUrl};
pub use document::{Attachment, Document, SavedAttachment, UnsavedAttachment};
pub use error::{Error, ErrorResponse};
pub use path::{DatabaseName, DatabaseNameRef, DatabasePath, DatabasePathRef, DesignDocumentName,
               DesignDocumentNameRef, DesignDocumentPath, DesignDocumentPathRef, DocumentId,
               DocumentIdRef, DocumentPath, DocumentPathRef, IntoDatabasePath,
               IntoDesignDocumentPath, IntoDocumentPath, IntoViewPath, LocalDocumentName,
               LocalDocumentNameRef, NormalDocumentName, NormalDocumentNameRef, ViewName,
               ViewNameRef, ViewPath, ViewPathRef};
pub use revision::Revision;
pub use view::{ReducedView, UnreducedView, ViewResponse, ViewRow};
