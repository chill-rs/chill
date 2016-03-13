use Error;
use error::PathParseErrorKind;
use serde;
use std;

macro_rules! define_name_types {
    ($owning_type:ident, $borrowed_type:ident) => {

        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $owning_type {
            inner: String,
        }

        #[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $borrowed_type {
            inner: str,
        }
    }
}

macro_rules! impl_base_methods {
    ($owning_type:ident, $borrowed_type:ident, $arg_name:ident) => {

        impl std::fmt::Display for $owning_type {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                self.inner.fmt(formatter)
            }
        }

        impl AsRef<str> for $owning_type {
            fn as_ref(&self) -> &str {
                &self.inner
            }
        }

        impl AsRef<$borrowed_type> for $owning_type {
            fn as_ref(&self) -> &$borrowed_type {
                $borrowed_type::from_str_ref(&self.inner)
            }
        }

        impl<'a> From<&'a str> for $owning_type {
            fn from($arg_name: &'a str) -> Self {
                $owning_type { inner: $arg_name.to_owned() }
            }
        }

        impl From<String> for $owning_type {
            fn from($arg_name: String) -> Self {
                $owning_type { inner: $arg_name }
            }
        }

        impl From<$owning_type> for String {
            fn from($arg_name: $owning_type) -> Self {
                $arg_name.inner
            }
        }

        impl std::borrow::Borrow<$borrowed_type> for $owning_type {
            fn borrow(&self) -> &$borrowed_type {
                &self.inner.as_ref()
            }
        }

        impl $borrowed_type {
            fn from_str_ref($arg_name: &str) -> &$borrowed_type {
                unsafe { std::mem::transmute($arg_name) }
            }
        }

        impl std::fmt::Display for $borrowed_type {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                self.inner.fmt(formatter)
            }
        }

        impl AsRef<str> for $borrowed_type {
            fn as_ref(&self) -> &str {
                &self.inner
            }
        }

        impl ToOwned for $borrowed_type {
            type Owned = $owning_type;
            fn to_owned(&self) -> Self::Owned {
                $owning_type::from(self.inner.to_owned())
            }
        }

        impl AsRef<$borrowed_type> for str {
            fn as_ref(&self) -> &$borrowed_type {
                $borrowed_type::from_str_ref(self)
            }
        }

        impl AsRef<$borrowed_type> for String {
            fn as_ref(&self) -> &$borrowed_type {
                $borrowed_type::from_str_ref(self)
            }
        }

    }
}

define_name_types!(DatabaseName, DatabaseSegment);
impl_base_methods!(DatabaseName, DatabaseSegment, db_name);

define_name_types!(DocumentId, DocumentSegment);
impl_base_methods!(DocumentId, DocumentSegment, doc_id);

// FIXME: Test this.
impl serde::Serialize for DocumentSegment {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

// FIXME: Test this.
impl serde::Deserialize for DocumentId {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = DocumentId;

            fn visit_str<E>(&mut self, encoded: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                Ok(DocumentId::from(encoded))
            }
        }

        deserializer.deserialize(Visitor)
    }
}

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

pub trait DatabasePath {
    fn database_path(&self) -> Result<&DatabaseSegment, Error>;
}

impl DatabasePath for &'static str {
    fn database_path(&self) -> Result<&DatabaseSegment, Error> {
        let remaining = self;
        let db_name = try!(path_extract_final(remaining));
        Ok(db_name.as_ref())
    }
}

impl<T> DatabasePath for T
    where T: std::borrow::Borrow<DatabaseSegment>
{
    fn database_path(&self) -> Result<&DatabaseSegment, Error> {
        Ok(self.borrow())
    }
}

pub trait DocumentPath {
    fn document_path(&self) -> Result<(&DatabaseSegment, &DocumentSegment), Error>;
}

impl DocumentPath for &'static str {
    fn document_path(&self) -> Result<(&DatabaseSegment, &DocumentSegment), Error> {
        let remaining = self;
        let (db_name, remaining) = try!(path_extract_nonfinal(remaining));
        let doc_id = try!(path_extract_final(remaining));
        Ok((db_name.as_ref(), doc_id.as_ref()))
    }
}

impl<T, U> DocumentPath for (T, U)
    where T: std::borrow::Borrow<DatabaseSegment>,
          U: std::borrow::Borrow<DocumentSegment>
{
    fn document_path(&self) -> Result<(&DatabaseSegment, &DocumentSegment), Error> {
        Ok((self.0.borrow(), self.1.borrow()))
    }
}

#[cfg(test)]
mod tests {

    use Error;
    use error::PathParseErrorKind;
    use super::*;

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

    #[test]
    fn database_name_display() {
        let db_name = DatabaseName { inner: String::from("foo") };
        let got = format!("{}", db_name);
        assert_eq!("foo", got);
    }

    #[test]
    fn database_name_as_ref_str() {
        let db_name = DatabaseName { inner: String::from("foo") };
        let got: &str = db_name.as_ref();
        assert_eq!("foo", got);
    }

    #[test]
    fn database_name_as_ref_segment() {
        let db_name = DatabaseName { inner: String::from("foo") };
        let got: &DatabaseSegment = db_name.as_ref();
        assert_eq!("foo", got.as_ref());
    }

    #[test]
    fn database_name_from_str_ref() {
        let expected = DatabaseName { inner: String::from("foo") };
        let got = DatabaseName::from("foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn database_name_from_string() {
        let expected = DatabaseName { inner: String::from("foo") };
        let got = DatabaseName::from(String::from("foo"));
        assert_eq!(expected, got);
    }

    #[test]
    fn string_from_database_name() {
        let db_name = DatabaseName { inner: String::from("foo") };
        let got = String::from(db_name);
        assert_eq!("foo", got);
    }

    #[test]
    fn database_name_borrow() {
        use std::borrow::Borrow;
        let db_name = DatabaseName { inner: String::from("foo") };
        let got: &DatabaseSegment = db_name.borrow();
        assert_eq!("foo", got.as_ref());
    }

    #[test]
    fn database_segment_display() {
        let db_name: &DatabaseSegment = "foo".as_ref();
        let got = format!("{}", db_name);
        assert_eq!("foo", got);
    }

    #[test]
    fn str_as_database_segment_as_ref_str() {
        let db_name: &DatabaseSegment = "foo".as_ref();
        let got: &str = db_name.as_ref();
        assert_eq!("foo", got);
    }

    #[test]
    fn string_as_database_segment() {
        let underlying = String::from("foo");
        let db_name: &DatabaseSegment = underlying.as_ref();
        let got: &str = db_name.as_ref();
        assert_eq!("foo", got);
    }

    #[test]
    fn database_segment_to_owned() {
        let expected = DatabaseName { inner: String::from("foo") };
        let db_name: &DatabaseSegment = "foo".as_ref();
        let got = db_name.to_owned();
        assert_eq!(expected, got);
    }

    #[test]
    fn database_path_impl_by_static_str_ref() {
        fn f<P: DatabasePath>(_path: P) {}
        f("/foo");
    }

    #[test]
    fn database_path_impl_by_segment() {
        fn f<P: DatabasePath>(_path: P) {}
        let db_name = DatabaseSegment::from_str_ref("foo");
        f(db_name);
    }

    #[test]
    fn database_path_impl_by_name() {
        fn f<P: DatabasePath>(_path: P) {}
        let db_name = DatabaseName::from("foo");
        f(db_name);
    }

    #[test]
    fn database_path_from_static_str_ref_ok() {
        let expected = DatabaseSegment::from_str_ref("foo");
        let source = "/foo";
        let got = source.database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn database_path_from_static_str_ref_nok_no_leading_slash() {
        match "foo".database_path() {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_from_static_str_ref_nok_empty_database_name() {
        match "/".database_path() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_from_static_str_ref_nok_too_many_segments() {
        match "/foo/bar".database_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_from_borrow_ok() {
        let expected = DatabaseSegment::from_str_ref("foo");
        let source = DatabaseSegment::from_str_ref("foo");
        let got = source.database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_display() {
        let doc_id = DocumentId { inner: String::from("foo") };
        let got = format!("{}", doc_id);
        assert_eq!("foo", got);
    }

    #[test]
    fn document_id_as_ref_str() {
        let doc_id = DocumentId { inner: String::from("foo") };
        let got: &str = doc_id.as_ref();
        assert_eq!("foo", got);
    }

    #[test]
    fn document_id_as_ref_segment() {
        let doc_id = DocumentId { inner: String::from("foo") };
        let got: &DocumentSegment = doc_id.as_ref();
        assert_eq!("foo", got.as_ref());
    }

    #[test]
    fn document_id_from_str_ref() {
        let expected = DocumentId { inner: String::from("foo") };
        let got = DocumentId::from("foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_string() {
        let expected = DocumentId { inner: String::from("foo") };
        let got = DocumentId::from(String::from("foo"));
        assert_eq!(expected, got);
    }

    #[test]
    fn string_from_document_id() {
        let doc_id = DocumentId { inner: String::from("foo") };
        let got = String::from(doc_id);
        assert_eq!("foo", got);
    }

    #[test]
    fn document_id_borrow() {
        use std::borrow::Borrow;
        let doc_id = DocumentId { inner: String::from("foo") };
        let got: &DocumentSegment = doc_id.borrow();
        assert_eq!("foo", got.as_ref());
    }

    #[test]
    fn document_segment_display() {
        let doc_id: &DocumentSegment = "foo".as_ref();
        let got = format!("{}", doc_id);
        assert_eq!("foo", got);
    }

    #[test]
    fn str_as_document_segment_as_ref_str() {
        let doc_id: &DocumentSegment = "foo".as_ref();
        let got: &str = doc_id.as_ref();
        assert_eq!("foo", got);
    }

    #[test]
    fn string_as_document_segment() {
        let underlying = String::from("foo");
        let doc_id: &DocumentSegment = underlying.as_ref();
        let got: &str = doc_id.as_ref();
        assert_eq!("foo", got);
    }

    #[test]
    fn document_segment_to_owned() {
        let expected = DocumentId { inner: String::from("foo") };
        let doc_id: &DocumentSegment = "foo".as_ref();
        let got = doc_id.to_owned();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_impl_by_static_str_ref() {
        fn f<P: DocumentPath>(_path: P) {}
        f("/foo");
    }

    #[test]
    fn document_path_impl_by_segments() {
        fn f<P: DocumentPath>(_path: P) {}
        let db_name = DatabaseSegment::from_str_ref("foo");
        let doc_id = DocumentSegment::from_str_ref("bar");
        f((db_name, doc_id));
    }

    #[test]
    fn document_path_impl_by_names() {
        fn f<P: DocumentPath>(_path: P) {}
        let db_name = DatabaseName::from("foo");
        let doc_id = DocumentId::from("bar");
        f((db_name, doc_id));
    }

    #[test]
    fn document_path_from_static_str_ref_ok() {
        let expected = (DatabaseSegment::from_str_ref("foo"),
                        DocumentSegment::from_str_ref("bar"));
        let source = "/foo/bar";
        let got = source.document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_from_static_str_ref_nok_no_leading_slash() {
        match "foo/bar".document_path() {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn document_path_from_static_str_ref_nok_too_few_segments() {
        match "/foo".document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooFewSegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn document_path_from_static_str_ref_nok_too_many_segments() {
        match "/foo/bar/qux".document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn document_path_from_borrow_ok() {
        let expected = (DatabaseSegment::from_str_ref("foo"),
                        DocumentSegment::from_str_ref("bar"));
        let source = (DatabaseSegment::from_str_ref("foo"),
                      DocumentSegment::from_str_ref("bar"));
        let got = source.document_path().unwrap();
        assert_eq!(expected, got);
    }
}
