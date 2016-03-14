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

macro_rules! define_name_type {
    ($type_name:ident, $arg_name:ident) => {

        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $type_name {
            inner: String,
        }

        impl std::fmt::Display for $type_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                self.inner.fmt(formatter)
            }
        }

        impl<'a> From<&'a str> for $type_name {
            fn from($arg_name: &'a str) -> Self {
                $type_name { inner: String::from($arg_name) }
            }
        }

        impl From<String> for $type_name {
            fn from($arg_name: String) -> Self {
                $type_name { inner: $arg_name }
            }
        }

        impl From<$type_name> for String {
            fn from($arg_name: $type_name) -> Self {
                $arg_name.inner
            }
        }
    }
}

macro_rules! impl_name_serialization {
    ($type_name:ident) => {
        impl serde::Serialize for $type_name {
            fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
                where S: serde::Serializer
            {
                self.inner.serialize(serializer)
            }
        }
    }
}

macro_rules! impl_name_deserialization {
    ($type_name:ident) => {
        impl serde::Deserialize for $type_name {
            fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
                where D: serde::Deserializer
            {
                struct Visitor;

                impl serde::de::Visitor for Visitor {
                    type Value = $type_name;

                    fn visit_str<E>(&mut self, encoded: &str) -> Result<Self::Value, E>
                        where E: serde::de::Error
                    {
                        Ok($type_name::from(encoded))
                    }

                    fn visit_string<E>(&mut self, encoded: String) -> Result<Self::Value, E>
                        where E: serde::de::Error
                    {
                        Ok($type_name::from(encoded))
                    }
                }

                deserializer.deserialize(Visitor)
            }
        }
    }
}

define_name_type!(DatabaseName, db_name);
define_name_type!(DocumentName, doc_name);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DocumentId {
    #[doc(hidden)]
    Normal(DocumentName),

    #[doc(hidden)]
    Design(DocumentName),

    #[doc(hidden)]
    Local(DocumentName),
}

impl DocumentId {
    fn prefix_as_str(&self) -> Option<&'static str> {
        match self {
            &DocumentId::Normal(_) => None,
            &DocumentId::Design(_) => Some("_design"),
            &DocumentId::Local(_) => Some("_local"),
        }
    }

    fn name_as_str(&self) -> &str {
        match self {
            &DocumentId::Normal(ref doc_name) => &doc_name.inner,
            &DocumentId::Design(ref doc_name) => &doc_name.inner,
            &DocumentId::Local(ref doc_name) => &doc_name.inner,
        }
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            &DocumentId::Normal(ref doc_name) => doc_name.fmt(formatter),
            &DocumentId::Design(ref doc_name) => write!(formatter, "_design/{}", doc_name),
            &DocumentId::Local(ref doc_name) => write!(formatter, "_local/{}", doc_name),
        }
    }
}

impl<'a> From<&'a str> for DocumentId {
    fn from(doc_id: &'a str) -> Self {

        let design_prefix = "_design/";
        let local_prefix = "_local/";

        if doc_id.starts_with(design_prefix) {
            DocumentId::Design(DocumentName::from(&doc_id[design_prefix.len()..]))
        } else if doc_id.starts_with(local_prefix) {
            DocumentId::Local(DocumentName::from(&doc_id[local_prefix.len()..]))
        } else {
            DocumentId::Normal(DocumentName::from(doc_id))
        }
    }
}

impl From<String> for DocumentId {
    fn from(doc_id: String) -> Self {
        // FIXME: Don't throw away the String, which leads to an extra
        // allocation.
        Self::from(doc_id.as_str())
    }
}

impl serde::Serialize for DocumentId {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        self.to_string().serialize(serializer)
    }
}

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

            fn visit_string<E>(&mut self, encoded: String) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                Ok(DocumentId::from(encoded))
            }
        }

        deserializer.deserialize(Visitor)
    }
}

// FIXME: Eliminate the necessity of ownership in DatabasePath.

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct DatabasePath {
    db_name: DatabaseName,
}

impl DatabasePath {
    #[cfg(test)]
    fn parse(path: &str) -> Result<Self, Error> {
        path.parse()
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DatabasePathIter {
        DatabasePathIter::DatabaseName(&self)
    }
}

impl std::str::FromStr for DatabasePath {
    type Err = Error;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let db_name = try!(path_extract_final(path));
        Ok(DatabasePath { db_name: DatabaseName::from(db_name) })
    }
}

pub enum DatabasePathIter<'a> {
    DatabaseName(&'a DatabasePath),
    Done,
}

impl<'a> Iterator for DatabasePathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self {
            &mut DatabasePathIter::DatabaseName(path) => {
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

pub trait IntoDatabasePath {
    fn into_database_path(self) -> Result<DatabasePath, Error>;
}

impl IntoDatabasePath for &'static str {
    fn into_database_path(self) -> Result<DatabasePath, Error> {
        self.parse()
    }
}

impl IntoDatabasePath for DatabasePath {
    fn into_database_path(self) -> Result<DatabasePath, Error> {
        Ok(self)
    }
}

impl IntoDatabasePath for DatabaseName {
    fn into_database_path(self) -> Result<DatabasePath, Error> {
        Ok(DatabasePath { db_name: self })
    }
}

// FIXME: Eliminate the necessity of ownership in DocumentPath.

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct DocumentPath {
    db_name: DatabaseName,
    doc_id: DocumentId,
}

impl DocumentPath {
    #[doc(hidden)]
    pub fn new_from_database_path_and_document_id(db_path: DatabasePath,
                                                  doc_id: DocumentId)
                                                  -> Self {
        DocumentPath {
            db_name: db_path.db_name,
            doc_id: doc_id,
        }
    }

    #[doc(hidden)]
    pub fn new_from_database_name_and_document_id(db_name: DatabaseName,
                                                  doc_id: DocumentId)
                                                  -> Self {
        DocumentPath {
            db_name: db_name,
            doc_id: doc_id,
        }
    }

    #[doc(hidden)]
    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    #[doc(hidden)]
    pub fn document_id(&self) -> &DocumentId {
        &self.doc_id
    }

    #[doc(hidden)]
    pub fn parse(path: &str) -> Result<Self, Error> {
        path.parse()
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DocumentPathIter {
        DocumentPathIter::DatabaseName(&self)
    }
}

impl std::str::FromStr for DocumentPath {
    type Err = Error;

    fn from_str(path: &str) -> Result<Self, Self::Err> {

        let remaining = path;
        let (db_name, remaining) = try!(path_extract_nonfinal(remaining));

        // The document id type is unusual in that it has a variable number of
        // segments.

        let design_prefix = "/_design/";
        let local_prefix = "/_local/";

        let doc_id = if remaining.starts_with(design_prefix) {
            let doc_name = try!(path_extract_final(&remaining[design_prefix.len() - 1..]));
            DocumentId::Design(DocumentName::from(doc_name))
        } else if remaining.starts_with(local_prefix) {
            let doc_name = try!(path_extract_final(&remaining[local_prefix.len() - 1..]));
            DocumentId::Local(DocumentName::from(doc_name))
        } else {
            let doc_name = try!(path_extract_final(remaining));
            DocumentId::Normal(DocumentName::from(doc_name))
        };

        Ok(DocumentPath {
            db_name: DatabaseName::from(db_name),
            doc_id: doc_id,
        })
    }
}

pub enum DocumentPathIter<'a> {
    DatabaseName(&'a DocumentPath),
    DocumentPrefix(&'a DocumentPath),
    DocumentName(&'a DocumentPath),
    Done,
}

impl<'a> Iterator for DocumentPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self {
            &mut DocumentPathIter::DatabaseName(path) => {
                (path.db_name.inner.as_str(),
                 DocumentPathIter::DocumentPrefix(path))
            }
            &mut DocumentPathIter::DocumentPrefix(path) => {
                match path.doc_id.prefix_as_str() {
                    None => (path.doc_id.name_as_str(), DocumentPathIter::Done),
                    Some(prefix) => (prefix, DocumentPathIter::DocumentName(path)),
                }
            }
            &mut DocumentPathIter::DocumentName(path) => {
                (path.doc_id.name_as_str(), DocumentPathIter::Done)
            }
            &mut DocumentPathIter::Done => {
                return None;
            }
        };

        *self = next;
        Some(item)
    }
}

pub trait IntoDocumentPath {
    fn into_document_path(self) -> Result<DocumentPath, Error>;
}

impl IntoDocumentPath for &'static str {
    fn into_document_path(self) -> Result<DocumentPath, Error> {
        self.parse()
    }
}

impl IntoDocumentPath for DocumentPath {
    fn into_document_path(self) -> Result<DocumentPath, Error> {
        Ok(self)
    }
}

impl<T> IntoDocumentPath for (T, DocumentId)
    where T: IntoDatabasePath
{
    fn into_document_path(self) -> Result<DocumentPath, Error> {
        let db_path = try!(self.0.into_database_path());
        Ok(DocumentPath {
            db_name: db_path.db_name,
            doc_id: self.1,
        })
    }
}

#[cfg(test)]
mod tests {

    use Error;
    use error::PathParseErrorKind;
    use serde;
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

    // Instead of testing each name type, we create a fake pair of types here
    // and test those. All name types are defined and implemented by macro, so
    // these tests cover all types.

    define_name_type!(FakeName, fake_name);

    #[test]
    fn fakename_display() {
        let got = format!("{}", FakeName::from("foo"));
        assert_eq!("foo", got);
    }

    #[test]
    fn fakename_from_str_ref() {
        let got = FakeName::from("foo");
        assert_eq!(FakeName { inner: String::from("foo") }, got);
    }

    #[test]
    fn fakename_from_string() {
        let got = FakeName::from(String::from("foo"));
        assert_eq!(FakeName { inner: String::from("foo") }, got);
    }

    impl_name_serialization!(FakeName);

    #[test]
    fn fakename_serialization() {
        let expected = serde_json::Value::String(String::from("foo"));
        let got = serde_json::to_value(&FakeName::from("foo"));
        assert_eq!(expected, got);
    }

    impl_name_deserialization!(FakeName);

    #[test]
    fn fakename_deserialization() {
        let got = serde_json::from_value(serde_json::Value::String(String::from("foo"))).unwrap();
        assert_eq!(FakeName::from("foo"), got);
    }

    #[test]
    fn document_id_prefix_as_str_normal() {
        assert_eq!(None, DocumentId::from("foo").prefix_as_str());
    }

    #[test]
    fn document_id_prefix_as_str_design() {
        assert_eq!(Some("_design"),
                   DocumentId::from("_design/foo").prefix_as_str());
    }

    #[test]
    fn document_id_prefix_as_str_local() {
        assert_eq!(Some("_local"),
                   DocumentId::from("_local/foo").prefix_as_str());
    }

    #[test]
    fn document_id_name_as_str_normal() {
        assert_eq!("foo", DocumentId::from("foo").name_as_str());
    }

    #[test]
    fn document_id_name_as_str_design() {
        assert_eq!("foo", DocumentId::from("_design/foo").name_as_str());
    }

    #[test]
    fn document_id_name_as_str_local() {
        assert_eq!("foo", DocumentId::from("_local/foo").name_as_str());
    }

    #[test]
    fn document_id_display_normal() {
        let got = format!("{}", DocumentId::Normal(DocumentName::from("foo")));
        assert_eq!("foo", got);
    }

    #[test]
    fn document_id_display_design() {
        let got = format!("{}", DocumentId::Design(DocumentName::from("foo")));
        assert_eq!("_design/foo", got);
    }

    #[test]
    fn document_id_display_local() {
        let got = format!("{}", DocumentId::Local(DocumentName::from("foo")));
        assert_eq!("_local/foo", got);
    }

    #[test]
    fn document_id_from_str_ref_design() {
        let got = DocumentId::from("_design/foo");
        assert_eq!(DocumentId::Design(DocumentName::from("foo")), got);
    }

    #[test]
    fn document_id_from_str_ref_local() {
        let got = DocumentId::from("_local/foo");
        assert_eq!(DocumentId::Local(DocumentName::from("foo")), got);
    }

    #[test]
    fn document_id_from_str_ref_normal() {
        let got = DocumentId::from("foo");
        assert_eq!(DocumentId::Normal(DocumentName::from("foo")), got);
    }

    #[test]
    fn document_id_from_string_design() {
        let got = DocumentId::from(String::from("_design/foo"));
        assert_eq!(DocumentId::Design(DocumentName::from("foo")), got);
    }

    #[test]
    fn document_id_from_string_local() {
        let got = DocumentId::from(String::from("_local/foo"));
        assert_eq!(DocumentId::Local(DocumentName::from("foo")), got);
    }

    #[test]
    fn document_id_from_string_normal() {
        let got = DocumentId::from(String::from("foo"));
        assert_eq!(DocumentId::Normal(DocumentName::from("foo")), got);
    }

    #[test]
    fn document_id_serialize_normal() {
        let got = serde_json::to_value(&DocumentId::Normal(DocumentName::from("foo")));
        assert_eq!(serde_json::Value::String(String::from("foo")), got);
    }

    #[test]
    fn document_id_serialize_design() {
        let got = serde_json::to_value(&DocumentId::Design(DocumentName::from("foo")));
        assert_eq!(serde_json::Value::String(String::from("_design/foo")), got);
    }

    #[test]
    fn document_id_serialize_local() {
        let got = serde_json::to_value(&DocumentId::Local(DocumentName::from("foo")));
        assert_eq!(serde_json::Value::String(String::from("_local/foo")), got);
    }

    #[test]
    fn document_id_deserialize_ok_normal() {
        let got = serde_json::from_value(serde_json::Value::String(String::from("foo"))).unwrap();
        assert_eq!(DocumentId::Normal(DocumentName::from("foo")), got);
    }

    #[test]
    fn document_id_deserialize_ok_design() {
        let got = serde_json::from_value(serde_json::Value::String(String::from("_design/foo")))
                      .unwrap();
        assert_eq!(DocumentId::Design(DocumentName::from("foo")), got);
    }

    #[test]
    fn document_id_deserialize_ok_local() {
        let got = serde_json::from_value(serde_json::Value::String(String::from("_local/foo")))
                      .unwrap();
        assert_eq!(DocumentId::Local(DocumentName::from("foo")), got);
    }

    #[test]
    fn database_path_from_str_ok() {
        let got = "/foo".parse().unwrap();
        assert_eq!(DatabasePath { db_name: DatabaseName::from("foo") }, got);
    }

    #[test]
    fn database_path_from_str_nok_no_leading_slash() {
        use std::str::FromStr;
        match DatabasePath::from_str("foo") {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_from_str_nok_trailing_slash() {
        use std::str::FromStr;
        match DatabasePath::from_str("/foo/") {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_from_str_nok_empty_segment() {
        use std::str::FromStr;
        match DatabasePath::from_str("/") {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_from_str_nok_too_many_segments() {
        use std::str::FromStr;
        match DatabasePath::from_str("/foo/bar") {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_iter() {
        let expected = vec!["foo"];
        let db_path = DatabasePath::parse("/foo").unwrap();
        let got = db_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_from_str_ok_normal() {
        let got = "/foo/bar".parse().unwrap();
        let expected = DocumentPath {
            db_name: DatabaseName::from("foo"),
            doc_id: DocumentId::Normal(DocumentName::from("bar")),
        };
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_from_str_ok_design() {
        let got = "/foo/_design/bar".parse().unwrap();
        let expected = DocumentPath {
            db_name: DatabaseName::from("foo"),
            doc_id: DocumentId::Design(DocumentName::from("bar")),
        };
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_from_str_ok_local() {
        let got = "/foo/_local/bar".parse().unwrap();
        let expected = DocumentPath {
            db_name: DatabaseName::from("foo"),
            doc_id: DocumentId::Local(DocumentName::from("bar")),
        };
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_from_str_nok_no_leading_slash() {
        use std::str::FromStr;
        match DocumentPath::from_str("foo/bar") {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn document_path_from_str_nok_trailing_slash() {
        use std::str::FromStr;
        match DocumentPath::from_str("/foo/bar/") {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn document_path_from_str_nok_empty_segment() {
        use std::str::FromStr;
        match DocumentPath::from_str("/foo/") {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn document_path_from_str_nok_too_few_segments() {
        use std::str::FromStr;
        match DocumentPath::from_str("/foo") {
            Err(Error::PathParse(PathParseErrorKind::TooFewSegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn document_path_from_str_nok_too_many_segments() {
        use std::str::FromStr;
        match DocumentPath::from_str("/foo/bar/qux") {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn document_path_iter_normal() {
        let expected = vec!["foo", "bar"];
        let db_path = DocumentPath::parse("/foo/bar").unwrap();
        let got = db_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_iter_design() {
        let expected = vec!["foo", "_design", "bar"];
        let db_path = DocumentPath::parse("/foo/_design/bar").unwrap();
        let got = db_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_iter_local() {
        let expected = vec!["foo", "_local", "bar"];
        let db_path = DocumentPath::parse("/foo/_local/bar").unwrap();
        let got = db_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }
}
