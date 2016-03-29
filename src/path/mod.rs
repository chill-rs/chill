//! Types and traits for specifying databases, documents, views, etc.
//!
//! Chill provides a rich set of types with which an application may specify
//! CouchDB documents, views, attachments, and other database resources. By
//! using special types—and not plain strings—the application programmer
//! benefits from compile-time checks against many mundane mistakes, such as:
//!
//! * Misspellings,
//! * Invalid percent-encodings, and,
//! * Wrong types, such as specifying a normal document instead of a design
//!   document by omitting the `_design` path segment.
//!
//! This page explains these types and their underlying design principles. Let's
//! start with Chill's distinction between names, ids, and paths.
//!
//! # Names, ids, and paths
//!
//! Chill has three categories of types for specifying CouchDB resources:
//! **names**, **ids**, and **paths**. Knowing the differences between these
//! three categories is critical for using Chill effectively.
//!
//! * A **name** is a single URL path segment for a resource. For example, given
//!   the URL path `/foo/_design/bar/_view/qux` for a view, `foo` is the
//!   **database name**, `bar` is the **document name**, and `qux` is the
//!   **view name**.
//!
//! * An **id** uniquely describes a resource within a given database. For
//!   example, given the URL path `/foo/_design/bar/_view/qux` for a view,
//!   `_design/bar` is the **document id** and `bar/qux` is the **view id**. A
//!   document id combines a document type (i.e., _normal_, _design_, or
//!   _local_) with a document name, and a view id combines a (design) document
//!   name with a view name.
//!
//! * A **path** specifies the full path of a resource. For example, the URL
//!   path `/foo/_design/bar/_view/qux` is a **view path**.
//!
//! Each name, id, and path is further divided into an owning type and a
//! borrowing type.
//!
//! # Owning vs borrowing types
//!
//! For each name, id, and path, Chill provides an **owning type** and a
//! **borrowing type**. This distinction is similar to the `PathBuf` and `Path`
//! types in the Rust standard library. However, in Chill, the borrowing types
//! end with a -`Ref` suffix (e.g., `DatabaseNameRef`, `DocumentPathRef`), and
//! the owning types lack a suffix (e.g., `DatabaseName` and `DocumentPath`).
//!
//! ```
//! use chill::*;
//!
//! // Statically allocated:
//! let borrowing = DatabaseNameRef::from("foo");
//! assert_eq!("foo", borrowing.to_string());
//!
//! // Heap-allocated copy of `borrowing`:
//! let owning = DatabaseName::from(borrowing);
//! assert_eq!("foo", owning.to_string());
//!
//! // Borrowed from the heap, with a lifetime equivalent to `owning`:
//! let also_borrowing = DatabaseNameRef::from(&owning);
//! assert_eq!("foo", also_borrowing.to_string());
//! ```
//!
//! Another difference between Chill's borrowing types and the `Path` type in
//! the Standard Library is that Chill's borrowing types are _sized types_. This
//! means they don't need to be used behind a pointer such as `&` or `Box`.
//! However, because they implement `Copy` and have borrowing semantics, in many
//! ways they act like references. Consult the types' individual documentation
//! for details.
//!
//! By providing owning and borrowing types, Chill allows applications to
//! eliminate heap allocations in many cases. Most methods in Chill that have a
//! name, id, or path parameter use the borrowing type for that parameter,
//! meaning no allocation is necessary. Conversion traits make this happen
//! conveniently.
//!
//! # Conversion traits
//!
//! The basic conversion traits are `From` and `Into` in the Standard Library.
//! Names and ids—regardless whether they're owning or borrowing—implement
//! `From` liberally, whereas paths implement `From` conservatively. Each path
//! provides a custom conversion trait because path-parsing is fallible and may
//! return a parse error.
//!
//! Here are some examples:
//!
//! ```
//! use chill::*;
//!
//! // Construct a name or id from a &str:
//! let db_name = DatabaseNameRef::from("foo");
//! let db_name = DatabaseName::from("foo");
//! let doc_id = DocumentIdRef::from("_design/bar");
//! let doc_id = DocumentId::from("_design/bar");
//!
//! // Construct an owning name or id by moving a String into it:
//! let db_name = DatabaseName::from(String::from("foo"));
//! let doc_id = DocumentId::from(String::from("_design/bar"));
//!
//! // Convert between owning and borrowing types:
//! let a = DatabaseNameRef::from("foo"); // does not allocate
//! let b = DatabaseName::from(a);        // allocates on the heap
//! let c = DatabaseNameRef::from(&b);    // does not allocate
//!
//! // Document names are split into three subtypes: normal, design, and local.
//! let normal = NormalDocumentNameRef::from("foo");
//! let design = DesignDocumentNameRef::from("foo");
//! let local = LocalDocumentNameRef::from("foo");
//!
//! // Take advantage of the document subtypes to enforce the correct //
//! // path-segment prefix (with zero heap allocations!):
//! let doc_id = DocumentIdRef::from(normal);
//! assert_eq!("foo", doc_id.to_string());
//! let doc_id = DocumentIdRef::from(design);
//! assert_eq!("_design/foo", doc_id.to_string());
//! let doc_id = DocumentIdRef::from(local);
//! assert_eq!("_local/foo", doc_id.to_string());
//!
//! // Paths may be converted from a string literal and must begin with a slash.
//! let db_path = "/foo".into_database_path().unwrap();
//! assert_eq!("foo", db_path.database_name().to_string());
//! let doc_path = "/foo/_design/bar".into_document_path().unwrap();
//! assert_eq!("foo", doc_path.database_name().to_string());
//! assert_eq!("_design/bar", doc_path.document_id().to_string());
//!
//! // Path conversions may fail:
//! "foo".into_database_path().unwrap_err();      // no leading slash
//! "/foo/bar".into_database_path().unwrap_err(); // too many segments
//! "/foo".into_document_path().unwrap_err();     // too few segments
//! ```
//!
//! Path conversions are special and deserve their own attention.
//!
//! # Path conversions
//!
//! Unlike with names and ids, an application may construct a path from a static
//! string but not from a runtime string. Instead, each path type provides a
//! custom conversion trait (e.g., `IntoDocumentPath`) that allows the
//! application to construct a path from constituent parts.
//!
//! ```
//! use chill::*;
//!
//! // From a static string:
//! let source = "/foo/_design/bar";
//! let doc_path = source.into_document_path().unwrap();
//! assert_eq!("foo", doc_path.database_name().to_string());
//! assert_eq!("_design/bar", doc_path.document_id().to_string());
//!
//! // From constituent parts:
//! let source = (DatabaseNameRef::from("foo"),
//!               DesignDocumentNameRef::from("bar"));
//! let doc_path = source.into_document_path().unwrap();
//! assert_eq!("foo", doc_path.database_name().to_string());
//! assert_eq!("_design/bar", doc_path.document_id().to_string());
//!
//! // From constituent parts, which can be strings:
//! let source = ("/foo", "_design/bar");
//! let doc_path = source.into_document_path().unwrap();
//! assert_eq!("foo", doc_path.database_name().to_string());
//! assert_eq!("_design/bar", doc_path.document_id().to_string());
//! ```
//!
//! The idea here is that Chill prevents the application from concatenating URL
//! path segments to form a path. This eliminates corner cases related to
//! percent-encoding. Consider this example:
//!
//! ```
//! use chill::*;
//!
//! let source = ("/foo", "bar/qux");
//! let db_path = source.into_document_path().unwrap();
//!
//! // URI path: /foo/bar%2Fqux
//! ```
//!
//! CouchDB allows `/` as a valid character in a document name, though it must
//! be percent-encoded as `%2F` when serialized in the request line of an HTTP
//! request.
//!
//! By forcing the application to construct the path from constituent parts,
//! Chill guarantees that percent-encoding is correct. But what happens when
//! parsing a static string?
//!
//! ```
//! use chill::*;
//!
//! let source = "/foo/bar%2Fqux";
//! let db_path = source.into_document_path().unwrap();
//!
//! // URI path: /foo/bar%252Fqux
//! ```
//!
//! The program compiles but doesn't do what's expected. Instead, it
//! percent-encodes the `%` character (as `%25`) because `%` is also a valid
//! character for document names and Chill has no way to disambiguate.
//!
//! Chill could make static-string-conversions illegal and eliminate this corner
//! case, but Chill's API would be less ergonomic. This is a case where
//! convenience trumps type-safety. Instead, the application programmer must
//! abide this one simple rule:
//!
//! > **The application never observes a percent-encoded character.**
//!
//! If the programmer had followed this rule then they wouldn't have tried to
//! manually percent-encode the path. But what if they tried this:
//!
//! ```should_panic
//! use chill::*;
//!
//! let source = "/foo/bar/qux";
//! let db_path = source.into_document_path().unwrap(); // Panics!
//! ```
//!
//! The string `"/foo/bar/qux"` is an invalid document path because it contains
//! too many path segments. It turns out some paths cannot be expressed as
//! static strings and must instead be constructed from constituent parts.
//! However, such cases are rare.
//!
//! # Invalid names, ids, and paths
//!
//! CouchDB imposes many restrictions on resource names, and though Chill
//! catches some parse errors when constructing paths, Chill doesn't enforce
//! _validity_. For example, CouchDB requires all database names to begin with a
//! letter, but Chill will allow the following:
//!
//! ```
//! use chill::*;
//! let db_name = DatabaseNameRef::from("_not_a_valid_name");
//! ```
//!
//! This compiles, but if the application uses the database name in a CouchDB
//! action, then it will receive an error from the server.
//!
//! This is true for all names, ids, and paths. The rationale is that the
//! CouchDB server is the source of truth for validity, and Chill makes no
//! attempt to duplicate this functionality.
//!
//! It's also possible to do something like this:
//!
//! ```
//! use chill::*;
//! let source = ("/foo", NormalDocumentNameRef::from("_design/bar"));
//! let doc_path = source.into_document_path().unwrap();
//!
//! // URI path: /foo/_design%2Fbar
//! ```
//!
//! In this example, the application may have intended to construct a path for a
//! normal document, but Chill unambiguously constructed a path for a design
//! document. This is a loophole in the Chill type system and further shows how
//! Chill allows convenience to trump type-safety in some cases. Application
//! programmers should be mindful when converting from raw strings.

use Error;
use error::PathParseErrorKind;
use std;

#[derive(Debug)]
struct PathExtractor<'a> {
    path: &'a str,
}

#[derive(Debug)]
enum PathExtraction<'a> {
    Final(&'a str),
    Nonfinal(PathExtractor<'a>, &'a str),
}

impl<'a> PathExtractor<'a> {
    fn new(path: &'a str) -> Self {
        PathExtractor { path: path }
    }

    fn extract_final(self) -> Result<&'a str, Error> {
        if !self.path.starts_with('/') {
            return Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash));
        }
        let segment = &self.path[1..];
        if let Some(index) = segment.find('/') {
            if segment[index + 1..].is_empty() {
                return Err(Error::PathParse(PathParseErrorKind::TrailingSlash));
            }
            return Err(Error::PathParse(PathParseErrorKind::TooManySegments));
        }
        if segment.is_empty() {
            return Err(Error::PathParse(PathParseErrorKind::EmptySegment));
        }
        Ok(segment)
    }

    fn extract_nonfinal(&mut self) -> Result<&'a str, Error> {
        if !self.path.starts_with('/') {
            return Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash));
        }
        let index = try!(self.path[1..]
                             .find('/')
                             .ok_or(Error::PathParse(PathParseErrorKind::TooFewSegments)));
        let (segment, remaining) = self.path[1..].split_at(index);
        if segment.is_empty() {
            return Err(Error::PathParse(PathParseErrorKind::EmptySegment));
        }
        self.path = remaining;
        Ok(segment)
    }

    fn extract_any(mut self) -> Result<PathExtraction<'a>, Error> {
        match self.extract_nonfinal() {
            Ok(segment) => Ok(PathExtraction::Nonfinal(self, segment)),
            Err(Error::PathParse(PathParseErrorKind::TooFewSegments)) => {
                self.extract_final().map(|x| PathExtraction::Final(x))
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod extractor_tests {

    use Error;
    use error::PathParseErrorKind;
    use super::{PathExtraction, PathExtractor};

    #[test]
    fn final_and_nonfinal_ok() {
        let mut p = PathExtractor::new("/foo/bar/qux");
        assert_eq!("foo", p.extract_nonfinal().unwrap());
        assert_eq!("bar", p.extract_nonfinal().unwrap());
        assert_eq!("qux", p.extract_final().unwrap());
    }

    #[test]
    fn any_ok() {
        let p = PathExtractor::new("/foo/bar/qux");
        let p = match p.extract_any() {
            Ok(PathExtraction::Nonfinal(p, "foo")) => p,
            x @ _ => unexpected_result!(x),
        };
        let p = match p.extract_any() {
            Ok(PathExtraction::Nonfinal(p, "bar")) => p,
            x @ _ => unexpected_result!(x),
        };
        match p.extract_any() {
            Ok(PathExtraction::Final("qux")) => (),
            x @ _ => unexpected_result!(x),
        };
    }

    #[test]
    fn nonfinal_nok_is_empty_string() {
        match PathExtractor::new("").extract_nonfinal() {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn nonfinal_nok_has_no_leading_slash() {
        match PathExtractor::new("foo/bar").extract_nonfinal() {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn nonfinal_nok_is_final() {
        match PathExtractor::new("/foo").extract_nonfinal() {
            Err(Error::PathParse(PathParseErrorKind::TooFewSegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn nonfinal_nok_is_empty_segment() {
        match PathExtractor::new("//foo").extract_nonfinal() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn final_nok_is_empty_string() {
        match PathExtractor::new("").extract_final() {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn final_nok_is_empty_segment() {
        match PathExtractor::new("/").extract_final() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn final_nok_has_extra_segment() {
        match PathExtractor::new("/foo/bar").extract_final() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn final_nok_has_trailing_slash() {
        match PathExtractor::new("/foo/").extract_final() {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }
}

macro_rules! define_name_type_pair {
    ($owning_type:ident, $borrowing_type:ident, $arg_name:ident, #[$description:meta]) => {

        /// A reference to a
        #[$description]
        /// name, owned elsewhere.
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $borrowing_type<'a> {
            inner: &'a str,
        }

        /// A heap-allocated
        #[$description]
        /// name, owned and managed internally.
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $owning_type {
            inner: String,
        }

        impl<'a> $borrowing_type<'a> {
            fn new(s: &'a str) -> Self {
                $borrowing_type { inner: s }
            }
        }

        impl $owning_type {
            fn new(s: String) -> Self {
                $owning_type { inner: s }
            }

            pub fn as_ref(&self) -> $borrowing_type {
                $borrowing_type::new(&self.inner)
            }
        }

        impl<'a> AsRef<str> for $borrowing_type<'a> {
            fn as_ref(&self) -> &str {
                self.inner
            }
        }

        impl AsRef<str> for $owning_type {
            fn as_ref(&self) -> &str {
                &self.inner
            }
        }

        impl<'a> std::fmt::Display for $borrowing_type<'a> {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                self.inner.fmt(formatter)
            }
        }

        impl std::fmt::Display for $owning_type {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                self.inner.fmt(formatter)
            }
        }

        impl<'a> From<&'a str> for $borrowing_type<'a> {
            fn from(s: &'a str) -> Self {
                $borrowing_type::new(s)
            }
        }

        impl<'a> From<&'a str> for $owning_type {
            fn from(s: &'a str) -> Self {
                $owning_type::new(s.into())
            }
        }

        impl From<String> for $owning_type {
            fn from(s: String) -> Self {
                $owning_type::new(s)
            }
        }

        impl From<$owning_type> for String {
            fn from($arg_name: $owning_type) -> Self {
                $arg_name.inner
            }
        }

        impl<'a> From<&'a $owning_type> for $borrowing_type<'a> {
            fn from($arg_name: &'a $owning_type) -> Self {
                $borrowing_type::new(&$arg_name.inner)
            }
        }

        impl<'a> From<$borrowing_type<'a>> for $owning_type {
            fn from($arg_name: $borrowing_type<'a>) -> Self {
                $owning_type::new($arg_name.inner.into())
            }
        }
    }
}

define_name_type_pair!(DatabaseName, DatabaseNameRef, db_name, /** database */);
define_name_type_pair!(DesignDocumentName, DesignDocumentNameRef, db_name, /** design document */);
define_name_type_pair!(LocalDocumentName, LocalDocumentNameRef, db_name, /** local document */);
define_name_type_pair!(NormalDocumentName, NormalDocumentNameRef, db_name, /** normal document */);
define_name_type_pair!(ViewName, ViewNameRef, view_name, /** view */);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DocumentIdRef<'a> {
    #[doc(hidden)]
    Normal(NormalDocumentNameRef<'a>),

    #[doc(hidden)]
    Design(DesignDocumentNameRef<'a>),

    #[doc(hidden)]
    Local(LocalDocumentNameRef<'a>),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DocumentId {
    #[doc(hidden)]
    Normal(NormalDocumentName),

    #[doc(hidden)]
    Design(DesignDocumentName),

    #[doc(hidden)]
    Local(LocalDocumentName),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatabasePathRef<'a> {
    db_name: DatabaseNameRef<'a>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatabasePath {
    db_name: DatabaseName,
}

pub trait IntoDatabasePath<'a> {
    fn into_database_path(self) -> Result<DatabasePathRef<'a>, Error>;
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentPathRef<'a> {
    db_name: DatabaseNameRef<'a>,
    doc_id: DocumentIdRef<'a>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentPath {
    db_name: DatabaseName,
    doc_id: DocumentId,
}

pub trait IntoDocumentPath<'a> {
    fn into_document_path(self) -> Result<DocumentPathRef<'a>, Error>;
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DesignDocumentPathRef<'a> {
    db_name: DatabaseNameRef<'a>,
    ddoc_name: DesignDocumentNameRef<'a>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DesignDocumentPath {
    db_name: DatabaseName,
    ddoc_name: DesignDocumentName,
}

pub trait IntoDesignDocumentPath<'a> {
    fn into_design_document_path(self) -> Result<DesignDocumentPathRef<'a>, Error>;
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ViewPathRef<'a> {
    db_name: DatabaseNameRef<'a>,
    ddoc_name: DesignDocumentNameRef<'a>,
    view_name: ViewNameRef<'a>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ViewPath {
    db_name: DatabaseName,
    ddoc_name: DesignDocumentName,
    view_name: ViewName,
}

pub trait IntoViewPath<'a> {
    fn into_view_path(self) -> Result<ViewPathRef<'a>, Error>;
}

// The following submodules implement methods for the traits and types defined
// above. The rationale for splitting across modules is so that the
// documentation all appears in one place—this module—while allowing many of the
// implementation details to reside elsewhere.

mod database_path;
mod design_document_path;
mod document_id;
mod document_path;
mod view_path;
