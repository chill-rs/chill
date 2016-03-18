use Error;
use error::PathParseErrorKind;
use std;

fn path_extract_nonfinal(path: &str) -> Result<(&str, &str), Error> {
    if !path.starts_with('/') {
        return Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash));
    }
    let path = &path[1..];
    let index = try!(path.find('/').ok_or(Error::PathParse(PathParseErrorKind::TooFewSegments)));
    let (segment, remaining) = path.split_at(index);
    if segment.is_empty() {
        return Err(Error::PathParse(PathParseErrorKind::EmptySegment));
    }
    Ok((segment, remaining))
}

fn path_extract_final(path: &str) -> Result<&str, Error> {
    if !path.starts_with('/') {
        return Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash));
    }
    let segment = &path[1..];
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

macro_rules! impl_name_types {
    ($borrowed_type:ident, $owning_type:ident, $name_arg:ident) => {

        impl $borrowed_type {
            fn new(s: &str) -> &$borrowed_type {
                unsafe { std::mem::transmute(s) }
            }
        }

        impl AsRef<$borrowed_type> for $borrowed_type {
            fn as_ref(&self) -> &$borrowed_type {
                self
            }
        }

        impl std::fmt::Display for $borrowed_type {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                self.inner.fmt(formatter)
            }
        }

        impl ToOwned for $borrowed_type {
            type Owned = $owning_type;
            fn to_owned(&self) -> Self::Owned {
                $owning_type { inner: String::from(&self.inner) }
            }
        }

        impl AsRef<$borrowed_type> for $owning_type {
            fn as_ref(&self) -> &$borrowed_type {
                $borrowed_type::new(&self.inner)
            }
        }

        impl std::borrow::Borrow<$borrowed_type> for $owning_type {
            fn borrow(&self) -> &$borrowed_type {
                $borrowed_type::new(&self.inner)
            }
        }

        impl std::ops::Deref for $owning_type {
            type Target = $borrowed_type;
            fn deref(&self) -> &Self::Target {
                $borrowed_type::new(&self.inner)
            }
        }

        impl std::fmt::Display for $owning_type {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                self.inner.fmt(formatter)
            }
        }

        impl<'a> From<&'a str> for $owning_type {
            fn from(s: &'a str) -> Self {
                $owning_type { inner: String::from(s) }
            }
        }

        impl From<String> for $owning_type {
            fn from(s: String) -> Self {
                $owning_type { inner: s }
            }
        }

        impl From<$owning_type> for String {
            fn from($name_arg: $owning_type) -> Self {
                $name_arg.inner
            }
        }
    }
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatabaseName {
    inner: str,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatabaseNameBuf {
    inner: String,
}

impl_name_types!(DatabaseName, DatabaseNameBuf, db_name);

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DesignDocumentName {
    inner: str,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DesignDocumentNameBuf {
    inner: String,
}

impl_name_types!(DesignDocumentName, DesignDocumentNameBuf, design_doc_name);

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentName {
    inner: str,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentNameBuf {
    inner: String,
}

impl_name_types!(DocumentName, DocumentNameBuf, doc_name);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatabasePath<'a> {
    db_name: &'a DatabaseName,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatabasePathBuf {
    db_name_buf: DatabaseNameBuf,
}

pub trait IntoDatabasePath<'a> {
    fn into_database_path(self) -> Result<DatabasePath<'a>, Error>;
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DocumentId<'a> {
    #[doc(hidden)]
    Normal(&'a DocumentName),

    #[doc(hidden)]
    Design(&'a DesignDocumentName),

    #[doc(hidden)]
    Local(&'a DocumentName),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DocumentIdBuf {
    #[doc(hidden)]
    Normal(DocumentNameBuf),

    #[doc(hidden)]
    Design(DesignDocumentNameBuf),

    #[doc(hidden)]
    Local(DocumentNameBuf),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentPath<'a> {
    db_name: &'a DatabaseName,
    doc_id: DocumentId<'a>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentPathBuf {
    db_name_buf: DatabaseNameBuf,
    doc_id_buf: DocumentIdBuf,
}

pub trait IntoDocumentPath<'a> {
    fn into_document_path(self) -> Result<DocumentPath<'a>, Error>;
}

// The following submodules implement methods for traits and types. However, the
// definitions for these traits and types reside in this submodule, above. The
// rationale for the definitions being in this module is that it causes the
// doc-comment documentation to organize all name, id, and path types and traits
// into this module. Whereas, the submodules exist merely to break this module
// into smaller parts for better readability and navigation.

mod database_path;
mod document_id;
mod document_path;

#[cfg(test)]
mod tests {

    use Error;
    use error::PathParseErrorKind;
    use std;

    #[test]
    fn path_extract_nonfinal_ok() {
        let got = super::path_extract_nonfinal("/foo/bar").unwrap();
        assert_eq!(("foo", "/bar"), got);
    }

    #[test]
    fn path_extract_nonfinal_nok_empty() {
        match super::path_extract_nonfinal("") {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn path_extract_nonfinal_nok_without_leading_slash() {
        match super::path_extract_nonfinal("foo/bar") {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn path_extract_nonfinal_nok_as_final() {
        match super::path_extract_nonfinal("/foo") {
            Err(Error::PathParse(PathParseErrorKind::TooFewSegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn path_extract_nonfinal_nok_with_empty_segment() {
        match super::path_extract_nonfinal("//foo") {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn path_extract_final_ok() {
        let segment = super::path_extract_final("/foo").unwrap();
        assert_eq!("foo", segment);
    }

    #[test]
    fn path_extract_final_segment_nok_empty() {
        match super::path_extract_final("") {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn path_extract_final_segment_nok_with_empty_segment() {
        match super::path_extract_final("/") {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn path_extract_final_segment_with_extra_segment() {
        match super::path_extract_final("/foo/bar") {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn path_extract_final_segment_with_trailing_slash() {
        match super::path_extract_final("/foo/") {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    // Instead of testing each pair of name types, we create a fake pair of
    // types and test those. All name types are defined and implemented by
    // macro, so these tests should cover all types.

    #[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct FakeName {
        inner: str,
    }

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct FakeNameBuf {
        inner: String,
    }

    impl_name_types!(FakeName, FakeNameBuf, fake_name);

    #[test]
    fn fake_name_new() {
        let fake_name = FakeName::new("foo");
        assert_eq!("foo", &fake_name.inner);
    }

    #[test]
    fn fake_name_as_ref() {
        let fake_name = FakeName::new("foo");
        let got: &FakeName = fake_name.as_ref();
        assert_eq!("foo", &got.inner);
    }

    #[test]
    fn fake_name_display() {
        let fake_name = FakeName::new("foo");
        let got = format!("{}", fake_name);
        assert_eq!("foo", got);
    }

    #[test]
    fn fake_name_to_owned() {
        let fake_name = FakeName::new("foo");
        let owned = fake_name.to_owned();
        assert_eq!(FakeNameBuf::from("foo"), owned);
    }

    #[test]
    fn fake_name_buf_as_ref() {
        let fake_name_buf = FakeNameBuf::from("foo");
        let got: &FakeName = fake_name_buf.as_ref();
        assert_eq!("foo", &got.inner);
    }

    #[test]
    fn fake_name_buf_borrow() {
        use std::borrow::Borrow;
        let fake_name_buf = FakeNameBuf::from("foo");
        let got: &FakeName = fake_name_buf.borrow();
        assert_eq!("foo", &got.inner);
    }

    #[test]
    fn fake_name_buf_deref() {
        use std::ops::Deref;
        let fake_name_buf = FakeNameBuf::from("foo");
        let got = fake_name_buf.deref();
        assert_eq!("foo", &got.inner);
    }

    #[test]
    fn fake_name_buf_display() {
        let fake_name_buf = FakeNameBuf::from("foo");
        let got = format!("{}", fake_name_buf);
        assert_eq!("foo", got);
    }

    #[test]
    fn fake_name_buf_from_str_ref() {
        let fake_name_buf = FakeNameBuf::from("foo");
        assert_eq!(FakeNameBuf { inner: String::from("foo") }, fake_name_buf);
    }

    #[test]
    fn fake_name_buf_from_string() {
        let fake_name_buf = FakeNameBuf::from(String::from("foo"));
        assert_eq!(FakeNameBuf { inner: String::from("foo") }, fake_name_buf);
    }

    #[test]
    fn string_from_fake_name_buf() {
        let fake_name_buf = FakeNameBuf::from("foo");
        let got = String::from(fake_name_buf);
        assert_eq!("foo", got);
    }
}
