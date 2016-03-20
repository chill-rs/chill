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

// The following submodules implement methods for the traits and types defined
// above. The rationale for splitting across modules is so that the
// documentation all appears in one place—this module—while allowing many of the
// implementation details to reside elsewhere.

mod database_path;
mod document_id;
mod document_path;
