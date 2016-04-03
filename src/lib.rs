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
pub use path::{AttachmentName, AttachmentNameRef, AttachmentPath, AttachmentPathRef, DatabaseName,
               DatabaseNameRef, DatabasePath, DatabasePathRef, DesignDocumentName,
               DesignDocumentNameRef, DesignDocumentPath, DesignDocumentPathRef, DocumentId,
               DocumentIdRef, DocumentPath, DocumentPathRef, IntoAttachmentPath, IntoDatabasePath,
               IntoDesignDocumentPath, IntoDocumentPath, IntoViewPath, LocalDocumentName,
               LocalDocumentNameRef, NormalDocumentName, NormalDocumentNameRef, ViewName,
               ViewNameRef, ViewPath, ViewPathRef};
pub use revision::Revision;
pub use view::{ReducedView, UnreducedView, ViewResponse, ViewRow};

mod prelude_impl {
    pub use super::*;
    pub use document::{JsonDecodableDocument, WriteDocumentResponse};
    #[cfg(test)]
    pub use document::DocumentBuilder;
    pub use error::{PathParseErrorKind, TransportErrorKind};
    pub use transport::{Action, RequestOptions, Response, Transport};
    pub use transport::production::HyperTransport;
    #[cfg(test)]
    pub use transport::testing::{MockResponse, MockTransport};
    pub use view::{ViewResponseJsonable, ViewResponseBuilder};

    // The StatusCode type is too prevalent to exclude from the prelude--even
    // though the type isn't ours.
    pub use hyper::status::StatusCode;
}
