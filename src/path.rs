use Error;
use error::PathParseErrorKind;
use serde;
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

macro_rules! define_name_types {
    ($borrowed_type:ident, $owning_type:ident, $name_arg:ident) => {

        #[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $borrowed_type {
            inner: str,
        }

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

        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $owning_type {
            inner: String,
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

define_name_types!(DatabaseName, DatabaseNameBuf, db_name);
define_name_types!(DesignDocumentName, DesignDocumentNameBuf, design_doc_name);
define_name_types!(DocumentName, DocumentNameBuf, doc_name);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DocumentId<'a> {
    #[doc(hidden)]
    Normal(&'a DocumentName),

    #[doc(hidden)]
    Design(&'a DesignDocumentName),

    #[doc(hidden)]
    Local(&'a DocumentName),
}

impl<'a> DocumentId<'a> {
    fn design_prefix() -> &'static str {
        "_design"
    }

    fn local_prefix() -> &'static str {
        "_local"
    }

    fn has_prefix(&self) -> bool {
        self.prefix_as_str().is_some()
    }

    fn prefix_as_str(&self) -> Option<&'static str> {
        match self {
            &DocumentId::Normal(_) => None,
            &DocumentId::Design(_) => Some(DocumentId::design_prefix()),
            &DocumentId::Local(_) => Some(DocumentId::local_prefix()),
        }
    }

    fn name_as_str(&self) -> &'a str {
        match self {
            &DocumentId::Normal(doc_name) => &doc_name.inner,
            &DocumentId::Design(doc_name) => &doc_name.inner,
            &DocumentId::Local(doc_name) => &doc_name.inner,
        }
    }
}

impl<'a> std::fmt::Display for DocumentId<'a> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            &DocumentId::Normal(doc_name) => doc_name.fmt(formatter),
            &DocumentId::Design(doc_name) => {
                write!(formatter, "{}/{}", DocumentId::design_prefix(), doc_name)
            }
            &DocumentId::Local(doc_name) => {
                write!(formatter, "{}/{}", DocumentId::local_prefix(), doc_name)
            }
        }
    }
}

impl<'a> serde::Serialize for DocumentId<'a> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        self.to_string().serialize(serializer)
    }
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

impl DocumentIdBuf {
    fn as_document_id(&self) -> DocumentId {
        match self {
            &DocumentIdBuf::Normal(ref doc_name_buf) => DocumentId::Normal(&doc_name_buf),
            &DocumentIdBuf::Design(ref doc_name_buf) => DocumentId::Design(&doc_name_buf),
            &DocumentIdBuf::Local(ref doc_name_buf) => DocumentId::Local(&doc_name_buf),
        }
    }
}

impl std::fmt::Display for DocumentIdBuf {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        self.into_document_id().fmt(formatter)
    }
}

impl<'a> From<&'a str> for DocumentIdBuf {
    fn from(s: &'a str) -> Self {
        match s.into_document_id() {
            DocumentId::Normal(doc_name) => DocumentIdBuf::Normal(doc_name.to_owned()),
            DocumentId::Design(doc_name) => DocumentIdBuf::Design(doc_name.to_owned()),
            DocumentId::Local(doc_name) => DocumentIdBuf::Local(doc_name.to_owned()),
        }
    }
}

impl From<String> for DocumentIdBuf {
    fn from(s: String) -> Self {
        // FIXME: Don't convert to a &str. Doing so causes an extra heap
        // allocation for the common case of a normal document.
        DocumentIdBuf::from(s.as_str())
    }
}

impl<'a> From<DocumentId<'a>> for DocumentIdBuf {
    fn from(doc_id: DocumentId<'a>) -> Self {
        match doc_id { 
            DocumentId::Normal(doc_name) => DocumentIdBuf::Normal(doc_name.to_owned()),
            DocumentId::Design(doc_name) => DocumentIdBuf::Design(doc_name.to_owned()),
            DocumentId::Local(doc_name) => DocumentIdBuf::Local(doc_name.to_owned()),
        }
    }
}

impl serde::Deserialize for DocumentIdBuf {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = DocumentIdBuf;

            fn visit_str<E>(&mut self, encoded: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                Ok(DocumentIdBuf::from(encoded))
            }

            fn visit_string<E>(&mut self, encoded: String) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                Ok(DocumentIdBuf::from(encoded))
            }
        }

        deserializer.deserialize(Visitor)
    }
}

pub trait IntoDocumentId<'a> {
    fn into_document_id(self) -> DocumentId<'a>;
}

impl<'a> IntoDocumentId<'a> for &'a str {
    fn into_document_id(self) -> DocumentId<'a> {

        let design_prefix = DocumentId::design_prefix();
        let local_prefix = DocumentId::local_prefix();

        if self.starts_with(design_prefix) && self[design_prefix.len()..].starts_with('/') {
            DocumentId::Design(DesignDocumentName::new(&self[design_prefix.len() + 1..]))
        } else if self.starts_with(local_prefix) && self[local_prefix.len()..].starts_with('/') {
            DocumentId::Local(DocumentName::new(&self[local_prefix.len() + 1..]))
        } else {
            DocumentId::Normal(DocumentName::new(self))
        }
    }
}

impl<'a> IntoDocumentId<'a> for DocumentId<'a> {
    fn into_document_id(self) -> DocumentId<'a> {
        self
    }
}

impl<'a> IntoDocumentId<'a> for &'a DocumentIdBuf {
    fn into_document_id(self) -> DocumentId<'a> {
        match self {
            &DocumentIdBuf::Normal(ref doc_name_buf) => DocumentId::Normal(&doc_name_buf),
            &DocumentIdBuf::Design(ref doc_name_buf) => DocumentId::Design(&doc_name_buf),
            &DocumentIdBuf::Local(ref doc_name_buf) => DocumentId::Local(&doc_name_buf),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatabasePath<'a> {
    db_name: &'a DatabaseName,
}

#[doc(hidden)]
impl<'a> IntoIterator for DatabasePath<'a> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = DatabasePathIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DatabasePathIter::DatabaseName(self)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatabasePathBuf {
    db_name_buf: DatabaseNameBuf,
}

impl DatabasePathBuf {
    #[doc(hidden)]
    pub fn parse(s: &'static str) -> Result<Self, Error> {
        let path = try!(s.into_database_path());
        Ok(DatabasePathBuf { db_name_buf: path.db_name.to_owned() })
    }

    #[doc(hidden)]
    pub fn as_database_path(&self) -> DatabasePath {
        DatabasePath { db_name: &self.db_name_buf }
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DatabasePathIter {
        self.as_database_path().into_iter()
    }
}

pub enum DatabasePathIter<'a> {
    DatabaseName(DatabasePath<'a>),
    Done,
}

impl<'a> Iterator for DatabasePathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self {
            &mut DatabasePathIter::DatabaseName(ref path) => {
                (&path.db_name.inner, DatabasePathIter::Done)
            }
            &mut DatabasePathIter::Done => {
                return None;
            }
        };

        *self = next;
        Some(item)
    }
}

pub trait IntoDatabasePath<'a> {
    fn into_database_path(self) -> Result<DatabasePath<'a>, Error>;
}

impl<'a> IntoDatabasePath<'a> for &'static str {
    fn into_database_path(self) -> Result<DatabasePath<'a>, Error> {
        let db_name = try!(path_extract_final(self));
        Ok(DatabasePath { db_name: DatabaseName::new(db_name) })
    }
}

impl<'a> IntoDatabasePath<'a> for DatabasePath<'a> {
    fn into_database_path(self) -> Result<DatabasePath<'a>, Error> {
        Ok(self)
    }
}

impl<'a> IntoDatabasePath<'a> for &'a DatabasePathBuf {
    fn into_database_path(self) -> Result<DatabasePath<'a>, Error> {
        Ok(self.as_database_path())
    }
}

impl<'a, T: AsRef<DatabaseName> + ?Sized> IntoDatabasePath<'a> for &'a T {
    fn into_database_path(self) -> Result<DatabasePath<'a>, Error> {
        Ok(DatabasePath { db_name: self.as_ref() })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentPath<'a> {
    db_name: &'a DatabaseName,
    doc_id: DocumentId<'a>,
}

impl<'a> DocumentPath<'a> {
    #[doc(hidden)]
    pub fn database_name(&self) -> &'a DatabaseName {
        &self.db_name
    }

    #[doc(hidden)]
    pub fn document_id(&self) -> &DocumentId<'a> {
        &self.doc_id
    }
}

#[doc(hidden)]
impl<'a> IntoIterator for DocumentPath<'a> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = DocumentPathIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DocumentPathIter {
            doc_path: self,
            state: DocumentPathIterState::DatabaseName,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentPathBuf {
    db_name_buf: DatabaseNameBuf,
    doc_id_buf: DocumentIdBuf,
}

impl DocumentPathBuf {
    #[doc(hidden)]
    pub fn new_from_parts(db_name: DatabaseNameBuf, doc_id: DocumentIdBuf) -> Self {
        DocumentPathBuf {
            db_name_buf: db_name,
            doc_id_buf: doc_id,
        }
    }

    #[doc(hidden)]
    pub fn parse(s: &'static str) -> Result<Self, Error> {
        let path = try!(s.into_document_path());
        Ok(DocumentPathBuf {
            db_name_buf: path.db_name.to_owned(),
            doc_id_buf: DocumentIdBuf::from(path.doc_id),
        })
    }

    #[doc(hidden)]
    pub fn as_document_path(&self) -> DocumentPath {
        DocumentPath {
            db_name: &self.db_name_buf,
            doc_id: self.doc_id_buf.as_document_id(),
        }
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DocumentPathIter {
        self.as_document_path().into_iter()
    }

    pub fn database_name(&self) -> &DatabaseNameBuf {
        &self.db_name_buf
    }

    pub fn document_id(&self) -> &DocumentIdBuf {
        &self.doc_id_buf
    }
}

#[doc(hidden)]
impl<'a> From<DocumentPath<'a>> for DocumentPathBuf {
    fn from(doc_path: DocumentPath<'a>) -> Self {
        DocumentPathBuf {
            db_name_buf: doc_path.db_name.to_owned(),
            doc_id_buf: doc_path.doc_id.into(),
        }
    }
}

pub struct DocumentPathIter<'a> {
    doc_path: DocumentPath<'a>,
    state: DocumentPathIterState,
}

enum DocumentPathIterState {
    DatabaseName,
    DocumentPrefix,
    DocumentName,
    Done,
}

impl<'a> Iterator for DocumentPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self.state {
            DocumentPathIterState::DatabaseName => {
                (&self.doc_path.db_name.inner,
                 if self.doc_path.doc_id.has_prefix() {
                    DocumentPathIterState::DocumentPrefix
                } else {
                    DocumentPathIterState::DocumentName
                })
            }
            DocumentPathIterState::DocumentPrefix => {
                (self.doc_path.doc_id.prefix_as_str().unwrap(),
                 DocumentPathIterState::DocumentName)
            }
            DocumentPathIterState::DocumentName => {
                (self.doc_path.doc_id.name_as_str(),
                 DocumentPathIterState::Done)
            }
            DocumentPathIterState::Done => {
                return None;
            }
        };

        self.state = next;
        Some(item)
    }
}

pub trait IntoDocumentPath<'a> {
    fn into_document_path(self) -> Result<DocumentPath<'a>, Error>;
}

impl<'a> IntoDocumentPath<'a> for &'static str {
    fn into_document_path(self) -> Result<DocumentPath<'static>, Error> {

        let (db_name, remaining) = try!(path_extract_nonfinal(self));

        // The document id type is unusual in that it has a variable number of
        // segments.

        // FIXME: Because this function is infallible, _all_ strings comprise a
        // document id. This includes strings such as "_design" and "_local". Is
        // this a good idea? At the least, it should be documented.

        let design_prefix = "/_design";
        let local_prefix = "/_local";

        let doc_id = if remaining.starts_with(design_prefix) {
            let doc_name = try!(path_extract_final(&remaining[design_prefix.len()..]));
            DocumentId::Design(DesignDocumentName::new(doc_name))
        } else if remaining.starts_with(local_prefix) {
            let doc_name = try!(path_extract_final(&remaining[local_prefix.len()..]));
            DocumentId::Local(DocumentName::new(doc_name))
        } else {
            let doc_name = try!(path_extract_final(remaining));
            DocumentId::Normal(DocumentName::new(doc_name))
        };

        Ok(DocumentPath {
            db_name: DatabaseName::new(db_name),
            doc_id: doc_id,
        })
    }
}

impl<'a> IntoDocumentPath<'a> for DocumentPath<'a> {
    fn into_document_path(self) -> Result<DocumentPath<'a>, Error> {
        Ok(self)
    }
}

impl<'a> IntoDocumentPath<'a> for &'a DocumentPathBuf {
    fn into_document_path(self) -> Result<DocumentPath<'a>, Error> {
        Ok(DocumentPath {
            db_name: DatabaseName::new(&self.db_name_buf.inner),
            doc_id: self.doc_id_buf.as_document_id(),
        })
    }
}

impl<'a, T, U> IntoDocumentPath<'a> for (T, U)
    where T: IntoDatabasePath<'a>,
          U: IntoDocumentId<'a>
{
    fn into_document_path(self) -> Result<DocumentPath<'a>, Error> {
        Ok(DocumentPath {
            db_name: try!(self.0.into_database_path()).db_name,
            doc_id: self.1.into_document_id(),
        })
    }
}

#[cfg(test)]
mod tests {

    use Error;
    use error::PathParseErrorKind;
    use serde_json;
    use std;
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

    // Instead of testing each pair of name types, we create a fake pair of
    // types and test those. All name types are defined and implemented by
    // macro, so these tests should cover all types.

    define_name_types!(FakeName, FakeNameBuf, fake_name);

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

    #[test]
    fn document_id_has_prefix_normal() {
        let doc_id = DocumentId::Normal(DocumentName::new("foo"));
        assert_eq!(false, doc_id.has_prefix());
    }

    #[test]
    fn document_id_has_prefix_design() {
        let doc_id = DocumentId::Design(DesignDocumentName::new("foo"));
        assert_eq!(true, doc_id.has_prefix());
    }

    #[test]
    fn document_id_has_prefix_local() {
        let doc_id = DocumentId::Local(DocumentName::new("foo"));
        assert_eq!(true, doc_id.has_prefix());
    }

    #[test]
    fn document_id_prefix_as_str_normal() {
        let doc_id = DocumentId::Normal(DocumentName::new("foo"));
        assert_eq!(None, doc_id.prefix_as_str());
    }

    #[test]
    fn document_id_prefix_as_str_design() {
        let doc_id = DocumentId::Design(DesignDocumentName::new("foo"));
        assert_eq!(Some("_design"), doc_id.prefix_as_str());
    }

    #[test]
    fn document_id_prefix_as_str_local() {
        let doc_id = DocumentId::Local(DocumentName::new("foo"));
        assert_eq!(Some("_local"), doc_id.prefix_as_str());
    }

    #[test]
    fn document_id_name_as_str_normal() {
        let doc_id = DocumentId::Normal(DocumentName::new("foo"));
        assert_eq!("foo", doc_id.name_as_str());
    }

    #[test]
    fn document_id_name_as_str_design() {
        let doc_id = DocumentId::Design(DesignDocumentName::new("foo"));
        assert_eq!("foo", doc_id.name_as_str());
    }

    #[test]
    fn document_id_name_as_str_local() {
        let doc_id = DocumentId::Local(DocumentName::new("foo"));
        assert_eq!("foo", doc_id.name_as_str());
    }

    #[test]
    fn document_id_display_normal() {
        let doc_id = DocumentId::Normal(DocumentName::new("foo"));
        assert_eq!("foo", format!("{}", doc_id));
    }

    #[test]
    fn document_id_display_design() {
        let doc_id = DocumentId::Design(DesignDocumentName::new("foo"));
        assert_eq!("_design/foo", format!("{}", doc_id));
    }

    #[test]
    fn document_id_display_local() {
        let doc_id = DocumentId::Local(DocumentName::new("foo"));
        assert_eq!("_local/foo", format!("{}", doc_id));
    }

    #[test]
    fn document_id_serialize_normal() {
        let expected = serde_json::Value::String(String::from("foo"));
        let doc_id = DocumentId::Normal(DocumentName::new("foo"));
        assert_eq!(expected, serde_json::to_value(&doc_id));
    }

    #[test]
    fn document_id_serialize_design() {
        let expected = serde_json::Value::String(String::from("_design/foo"));
        let doc_id = DocumentId::Design(DesignDocumentName::new("foo"));
        assert_eq!(expected, serde_json::to_value(&doc_id));
    }

    #[test]
    fn document_id_serialize_local() {
        let expected = serde_json::Value::String(String::from("_local/foo"));
        let doc_id = DocumentId::Local(DocumentName::new("foo"));
        assert_eq!(expected, serde_json::to_value(&doc_id));
    }

    #[test]
    fn document_id_buf_as_document_id_normal() {
        let doc_id_buf = DocumentIdBuf::Normal(DocumentNameBuf::from("foo"));
        let expected = DocumentId::Normal(DocumentName::new("foo"));
        assert_eq!(expected, doc_id_buf.as_document_id());
    }

    #[test]
    fn document_id_buf_as_document_id_design() {
        let doc_id_buf = DocumentIdBuf::Design(DesignDocumentNameBuf::from("foo"));
        let expected = DocumentId::Design(DesignDocumentName::new("foo"));
        assert_eq!(expected, doc_id_buf.as_document_id());
    }

    #[test]
    fn document_id_buf_as_document_id_local() {
        let doc_id_buf = DocumentIdBuf::Local(DocumentNameBuf::from("foo"));
        let expected = DocumentId::Local(DocumentName::new("foo"));
        assert_eq!(expected, doc_id_buf.as_document_id());
    }

    #[test]
    fn document_id_buf_display_normal() {
        let doc_id_buf = DocumentIdBuf::Normal(DocumentNameBuf::from("foo"));
        assert_eq!("foo", format!("{}", doc_id_buf));
    }

    #[test]
    fn document_id_buf_display_design() {
        let doc_id_buf = DocumentIdBuf::Design(DesignDocumentNameBuf::from("foo"));
        assert_eq!("_design/foo", format!("{}", doc_id_buf));
    }

    #[test]
    fn document_id_buf_display_local() {
        let doc_id_buf = DocumentIdBuf::Local(DocumentNameBuf::from("foo"));
        assert_eq!("_local/foo", format!("{}", doc_id_buf));
    }

    #[test]
    fn document_id_buf_from_str_ref_normal() {
        let expected = DocumentIdBuf::Normal(DocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from("foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_from_str_ref_design() {
        let expected = DocumentIdBuf::Design(DesignDocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from("_design/foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_from_str_ref_local() {
        let expected = DocumentIdBuf::Local(DocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from("_local/foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_from_string_normal() {
        let expected = DocumentIdBuf::Normal(DocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from(String::from("foo"));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_from_string_design() {
        let expected = DocumentIdBuf::Design(DesignDocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from(String::from("_design/foo"));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_from_string_local() {
        let expected = DocumentIdBuf::Local(DocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from(String::from("_local/foo"));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_from_document_id_normal() {
        let expected = DocumentIdBuf::Normal(DocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from(DocumentId::Normal(DocumentName::new("foo")));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_from_document_id_design() {
        let expected = DocumentIdBuf::Design(DesignDocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from(DocumentId::Design(DesignDocumentName::new("foo")));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_from_document_id_local() {
        let expected = DocumentIdBuf::Local(DocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from(DocumentId::Local(DocumentName::new("foo")));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_deserialize_normal() {
        let expected = DocumentIdBuf::Normal(DocumentNameBuf::from("foo"));
        let got = serde_json::from_value(serde_json::Value::String(String::from("foo"))).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_deserialize_design() {
        let expected = DocumentIdBuf::Design(DesignDocumentNameBuf::from("foo"));
        let got = serde_json::from_value(serde_json::Value::String(String::from("_design/foo")))
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_deserialize_local() {
        let expected = DocumentIdBuf::Local(DocumentNameBuf::from("foo"));
        let got = serde_json::from_value(serde_json::Value::String(String::from("_local/foo")))
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn str_ref_into_document_id_normal() {
        let expected = DocumentId::Normal(DocumentName::new("foo"));
        let got = "foo".into_document_id();
        assert_eq!(expected, got);
    }

    #[test]
    fn str_ref_into_document_id_normal_begins_with_design() {
        // This is an invalid document name, but our type should still exhibit
        // sane behavior.
        let expected = DocumentId::Normal(DocumentName::new("_designfoo"));
        let got = "_designfoo".into_document_id();
        assert_eq!(expected, got);
    }

    #[test]
    fn str_ref_into_document_id_normal_begins_with_local() {
        // This is an invalid document name, but our type should still exhibit
        // sane behavior.
        let expected = DocumentId::Normal(DocumentName::new("_localfoo"));
        let got = "_localfoo".into_document_id();
        assert_eq!(expected, got);
    }

    #[test]
    fn str_ref_into_document_id_design() {
        let expected = DocumentId::Design(DesignDocumentName::new("foo"));
        let got = "_design/foo".into_document_id();
        assert_eq!(expected, got);
    }

    #[test]
    fn str_ref_into_document_id_local() {
        let expected = DocumentId::Local(DocumentName::new("foo"));
        let got = "_local/foo".into_document_id();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_into_document_id() {
        let doc_id = DocumentId::Normal(DocumentName::new("foo"));
        assert_eq!(doc_id, doc_id.clone().into_document_id());
    }

    #[test]
    fn document_id_buf_into_document_id() {
        let expected = DocumentId::Normal(DocumentName::new("foo"));
        let doc_id_buf = DocumentIdBuf::Normal(DocumentNameBuf::from("foo"));
        assert_eq!(expected, doc_id_buf.into_document_id());
    }

    #[test]
    fn database_path_into_iter() {
        let got = "/foo".into_database_path().unwrap().into_iter().collect::<Vec<_>>();
        assert_eq!(vec!["foo"], got);
    }

    #[test]
    fn database_path_buf_parse_ok() {
        let expected = DatabasePathBuf { db_name_buf: DatabaseNameBuf::from("foo") };
        let got = DatabasePathBuf::parse("/foo").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn database_path_buf_parse_nok() {
        match DatabasePathBuf::parse("foo") {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_buf_as_database_path() {
        let expected = DatabasePath { db_name: &DatabaseName::new("foo") };
        let db_path_buf = DatabasePathBuf::parse("/foo").unwrap();
        let got = db_path_buf.as_database_path();
        assert_eq!(expected, got);
    }

    #[test]
    fn database_path_buf_iter() {
        let db_path_buf = DatabasePathBuf::parse("/foo").unwrap();
        let got = db_path_buf.iter().collect::<Vec<_>>();
        assert_eq!(vec!["foo"], got);
    }

    #[test]
    fn static_str_ref_into_database_path_ok() {
        let db_name_buf = DatabaseNameBuf::from("foo");
        let expected = DatabasePath { db_name: &db_name_buf };
        let got = "/foo".into_database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_into_database_path_nok_no_leading_slash() {
        match "foo".into_database_path() {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_database_path_nok_empty_database_name() {
        match "/".into_database_path() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_database_path_nok_trailing_slash() {
        match "/foo/".into_database_path() {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_database_path_nok_too_many_segments() {
        match "/foo/bar".into_database_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_buf_into_database_path_ok() {
        let db_name_buf = DatabaseNameBuf::from("foo");
        let db_path_buf = DatabasePathBuf { db_name_buf: db_name_buf.clone() };
        let expected = DatabasePath { db_name: &db_name_buf };
        let got = db_path_buf.into_database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn database_path_into_database_path_ok() {
        let db_name_buf = DatabaseNameBuf::from("foo");
        let db_path = DatabasePath { db_name: &db_name_buf };
        let expected = db_path.clone();
        let got = db_path.into_database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn database_name_buf_into_database_path_ok() {
        let db_name_buf = DatabaseNameBuf::from("foo");
        let expected = DatabasePath { db_name: &db_name_buf };
        let got = db_name_buf.into_database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_impls_into_database_path() {
        fn f<'a, P: IntoDatabasePath<'a>>(_: P) {}
        f("/foo");
    }

    #[test]
    fn database_path_impls_into_database_path() {
        fn f<'a, P: IntoDatabasePath<'a>>(_: P) {}
        f(&DatabasePathBuf { db_name_buf: DatabaseNameBuf::from("foo") });
    }

    #[test]
    fn database_name_buf_impls_into_database_path() {
        fn f<'a, P: IntoDatabasePath<'a>>(_: P) {}
        f(&DatabaseNameBuf::from("foo"));
    }

    #[test]
    fn database_name_impls_into_database_path() {
        fn f<'a, P: IntoDatabasePath<'a>>(_: P) {}
        f(DatabaseName::new("foo"));
    }

    #[test]
    fn document_path_into_iter_normal() {
        let doc_path = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let got = doc_path.into_iter().collect::<Vec<_>>();
        assert_eq!(vec!["foo", "bar"], got);
    }

    #[test]
    fn document_path_into_iter_design() {
        let doc_path = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let got = doc_path.into_iter().collect::<Vec<_>>();
        assert_eq!(vec!["foo", "_design", "bar"], got);
    }

    #[test]
    fn document_path_into_iter_local() {
        let doc_path = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let got = doc_path.into_iter().collect::<Vec<_>>();
        assert_eq!(vec!["foo", "_local", "bar"], got);
    }

    #[test]
    fn document_path_buf_new_from_parts() {
        let expected = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got =
            DocumentPathBuf::new_from_parts(DatabaseNameBuf::from("foo"),
                                            DocumentIdBuf::Normal(DocumentNameBuf::from("bar")));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_parse_normal() {
        let expected = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got = DocumentPathBuf::parse("/foo/bar").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_parse_design() {
        let expected = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Design(DesignDocumentNameBuf::from("bar")),
        };
        let got = DocumentPathBuf::parse("/foo/_design/bar").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_parse_local() {
        let expected = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Local(DocumentNameBuf::from("bar")),
        };
        let got = DocumentPathBuf::parse("/foo/_local/bar").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_as_document_path_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.as_document_path();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_as_document_path_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Design(DesignDocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.as_document_path();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_as_document_path_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Local(DocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.as_document_path();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_iter_normal() {
        let expected = vec!["foo", "bar"];
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_iter_design() {
        let expected = vec!["foo", "_design", "bar"];
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Design(DesignDocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_iter_local() {
        let expected = vec!["foo", "_local", "bar"];
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Local(DocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_from_document_path() {
        let expected = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got = DocumentPathBuf::from(DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        });
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_into_document_path_ok_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let got = "/foo/bar".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_into_document_path_ok_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let got = "/foo/_design/bar".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_into_document_path_ok_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let got = "/foo/_local/bar".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_into_document_path_nok_no_leading_slash() {
        match "foo/bar".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_database_only() {
        match "/foo".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooFewSegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_empty_document_id() {
        match "/foo/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_empty_design_document_id() {
        match "/foo/_design/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_empty_local_document_id() {
        match "/foo/_local/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_too_many_segments_normal() {
        match "/foo/bar/qux".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_too_many_segments_design() {
        match "/foo/_design/bar/qux".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_too_many_segments_local() {
        match "/foo/_local/bar/qux".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_trailing_slash_normal() {
        match "/foo/bar/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_trailing_slash_design() {
        match "/foo/_design/bar/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_trailing_slash_local() {
        match "/foo/_local/bar/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn into_document_path_with_document_id_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let got = ("/foo", DocumentId::Normal(DocumentName::new("bar")))
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_with_document_id_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let got = ("/foo", DocumentId::Design(DesignDocumentName::new("bar")))
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_with_document_id_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let got = ("/foo", DocumentId::Local(DocumentName::new("bar")))
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_with_document_id_buf_ref_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let doc_id_buf = DocumentIdBuf::Normal(DocumentNameBuf::from("bar"));
        let got = ("/foo", &doc_id_buf)
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_with_document_id_buf_ref_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let doc_id_buf = DocumentIdBuf::Design(DesignDocumentNameBuf::from("bar"));
        let got = ("/foo", &doc_id_buf)
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_with_document_id_buf_ref_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let doc_id_buf = DocumentIdBuf::Local(DocumentNameBuf::from("bar"));
        let got = ("/foo", &doc_id_buf)
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_buf_ref_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got = (&doc_path_buf).into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_buf_ref_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Design(DesignDocumentNameBuf::from("bar")),
        };
        let got = (&doc_path_buf).into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_buf_ref_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Local(DocumentNameBuf::from("bar")),
        };
        let got = (&doc_path_buf).into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let got = expected.clone().into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let got = expected.clone().into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let got = expected.clone().into_document_path().unwrap();
        assert_eq!(expected, got);
    }
}
