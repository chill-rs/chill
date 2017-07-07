extern crate base64;
extern crate futures;
extern crate mime;
extern crate regex;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tempdir;
extern crate url;
extern crate uuid;

/*
extern crate base64;
extern crate hyper;
#[macro_use(mime, __mime__ident_or_ext)]
*/

mod attachment;
mod client;
/*
mod design;
mod document;
*/
mod error;
mod revision;
/*
mod transport;
mod view;

pub mod action;
*/
pub mod path;
/*
pub mod testing;
*/

pub use attachment::{Attachment, SavedAttachment, UnsavedAttachment};
pub use client::IntoUrl;
/*
pub use design::{Design, DesignBuilder, ViewFunction};
pub use document::Document;
*/

pub use error::{Error, ErrorResponse};
pub use path::{AttachmentName, AttachmentPath, DatabaseName, DatabasePath, DesignDocumentName, DesignDocumentPath,
               DocumentId, DocumentPath, IntoAttachmentPath, IntoDatabasePath, IntoDesignDocumentPath,
               IntoDocumentPath, IntoViewPath, LocalDocumentName, NormalDocumentName, ViewName, ViewPath};
pub use revision::Revision;
/*
pub use view::{ViewResponse, ViewRow};
*/
