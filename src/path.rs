use {Error, serde, std};

#[doc(hidden)]
#[derive(Debug)]
pub enum PathParseError {
    BadSegment(&'static str),
    EmptySegment,
    NoLeadingSlash,
    TooFewSegments,
    TooManySegments,
    TrailingSlash,
}

impl std::fmt::Display for PathParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        use self::PathParseError::*;
        match self {
            &BadSegment(expected) => write!(formatter, "Segment is bad, expected {:?}", expected),
            &EmptySegment => write!(formatter, "Path segment is empty"),
            &NoLeadingSlash => write!(formatter, "Path does not begin with a slash"),
            &TooFewSegments => write!(formatter, "Too few path segments"),
            &TooManySegments => write!(formatter, "Too many path segments"),
            &TrailingSlash => write!(formatter, "Path ends with a slash"),
        }
    }
}


const DESIGN_PREFIX: &'static str = "_design";
const LOCAL_PREFIX: &'static str = "_local";
const VIEW_PREFIX: &'static str = "_view";

fn percent_encode(x: &str) -> String {
    use url::percent_encoding;
    percent_encoding::percent_encode(x.as_bytes(), percent_encoding::PATH_SEGMENT_ENCODE_SET).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn percent_encode() {
        use super::percent_encode;
        assert_eq!("", percent_encode(""));
        assert_eq!("alpha", percent_encode("alpha"));
        assert_eq!("%2F%20%25%3F", percent_encode("/ %?"));
    }
}

// PathExtractor is a utility for parsing a path string into its constituent
// segments. E.g., the path string "/alpha/bravo" may be extracted into two
// segments, "alpha" and "bravo".
//
// One benefit of using PathExtractor is that it provides consistent
// error-reporting.
//
#[derive(Debug, PartialEq)]
struct PathExtractor<'a> {
    path: &'a str,
}

impl<'a> PathExtractor<'a> {
    fn begin(path: &'a str) -> Result<Self, Error> {

        if !path.starts_with('/') {
            return Err(PathParseError::NoLeadingSlash)?;
        }

        Ok(PathExtractor { path: path })
    }

    fn end(self) -> Result<(), Error> {
        match self.path {
            "" => Ok(()),
            "/" => Err(PathParseError::TrailingSlash)?,
            _ => Err(PathParseError::TooManySegments)?,
        }
    }

    fn extract_nonempty(&mut self) -> Result<&'a str, Error> {

        if self.path.is_empty() {
            return Err(PathParseError::TooFewSegments)?;
        }

        assert!(self.path.starts_with('/'));
        let current = &self.path['/'.len_utf8()..];

        if current.is_empty() {
            return Err(PathParseError::TooFewSegments)?;
        }

        Ok(match current.find('/') {
            Some(0) => {
                return Err(PathParseError::EmptySegment)?;
            }
            Some(slash_index) => {
                let segment = &current[..slash_index];
                self.path = &current[slash_index..];
                segment
            }
            None => {
                let segment = current;
                self.path = &current[current.len()..];
                segment
            }
        })
    }

    fn extract_literal(&mut self, literal: &'static str) -> Result<(), Error> {

        if self.path.is_empty() {
            return Err(PathParseError::TooFewSegments)?;
        }

        assert!(self.path.starts_with('/'));
        let current = &self.path['/'.len_utf8()..];

        if current.is_empty() {
            return Err(PathParseError::TooFewSegments)?;
        }

        match current.find('/') {
            Some(slash_index) if &current[..slash_index] == literal => (),
            None if current == literal => (),
            _ => {
                return Err(PathParseError::BadSegment(literal))?;
            }
        }

        self.path = &current[literal.len()..];

        Ok(())
    }
}

#[cfg(test)]
mod path_extractor_tests {

    use super::{PathExtractor, PathParseError};
    use Error;

    #[test]
    fn begin() {

        macro_rules! nok {
            ($input:expr) => {
                match PathExtractor::begin($input) {
                    Err(Error::PathParse{ inner: PathParseError::NoLeadingSlash }) => (),
                    x => panic!("Got unexpected result {:?}", x),
                }
            }
        }

        // OK case:
        PathExtractor { path: "" }.end().unwrap();

        nok!("");
        nok!("alpha");
        nok!("alpha/bravo");
    }

    #[test]
    fn end() {

        macro_rules! nok {
            ($input:expr, $expected_error_kind:pat) => {{

                let path_extractor = PathExtractor {
                    path: $input,
                };

                match path_extractor.end() {
                    Err(Error::PathParse{ inner: $expected_error_kind }) => (),
                    x => panic!("Got unexpected result {:?}", x),
                }
            }}
        }

        nok!("/", PathParseError::TrailingSlash);
        nok!("//", PathParseError::TooManySegments);
        nok!("/alpha", PathParseError::TooManySegments);
    }

    #[test]
    fn extract_nonempty() {

        macro_rules! ok {
            ($input:expr, $expected_return:expr, $expected_remaining:expr) => {{

                let mut path_extractor = PathExtractor::begin($input).unwrap();
                let segment = path_extractor.extract_nonempty().unwrap();

                if $expected_return != segment {
                    panic!("Expected return value {:?}, got {:?}", $expected_return, segment);
                }

                if $expected_remaining != path_extractor.path {
                    panic!("Expected final path extractor state {:?}, got {:?}",
                           $expected_remaining,
                           path_extractor.path);
                }
            }}
        }

        macro_rules! nok {
            ($input:expr, $expected_error_kind:pat) => {{

                let mut path_extractor = PathExtractor { path: $input };

                match path_extractor.extract_nonempty() {
                    Err(Error::PathParse{ inner: $expected_error_kind }) => (),
                    x => panic!("Got unexpected result {:?}", x),
                }

                if $input != path_extractor.path {
                    panic!("Path extractor state was modified, expected {:?}, got {:?}",
                           $input,
                           path_extractor.path);
                }
            }}
        }

        ok!("/alpha", "alpha", "");
        ok!("/alpha/", "alpha", "/");
        ok!("/alpha/bravo", "alpha", "/bravo");

        nok!("", PathParseError::TooFewSegments);
        nok!("/", PathParseError::TooFewSegments);
        nok!("//", PathParseError::EmptySegment);
        nok!("//alpha", PathParseError::EmptySegment);
    }

    #[test]
    fn extract_literal() {

        macro_rules! ok {
            ($input:expr, $literal:expr, $expected_remaining:expr) => {{

                let mut path_extractor = PathExtractor::begin($input).unwrap();
                path_extractor.extract_literal($literal).unwrap();

                if $expected_remaining != path_extractor.path {
                    panic!("Expected final path extractor state {:?}, got {:?}",
                           $expected_remaining,
                           path_extractor.path);
                }
            }}
        }

        macro_rules! nok {
            ($input:expr, $literal:expr, $expected_error_kind:pat) => {{

                let mut path_extractor = PathExtractor { path: $input };

                match path_extractor.extract_literal($literal) {
                    Err(Error::PathParse{ inner: $expected_error_kind }) => (),
                    x => panic!("Got unexpected result {:?}", x),
                }

                if $input != path_extractor.path {
                    panic!("Path extractor state was modified, expected {:?}, got {:?}",
                           $input,
                           path_extractor.path);
                }
            }}
        }

        ok!("/alpha", "alpha", "");
        ok!("/alpha/", "alpha", "/");
        ok!("/alpha/bravo", "alpha", "/bravo");

        nok!("", "alpha", PathParseError::TooFewSegments);
        nok!("/", "alpha", PathParseError::TooFewSegments);
        nok!("//", "alpha", PathParseError::BadSegment("alpha"));
        nok!("//alpha", "alpha", PathParseError::BadSegment("alpha"));
        nok!("/alpha/bravo", "bravo", PathParseError::BadSegment("bravo"));
    }
}

macro_rules! define_name_type {
    ($type_name:ident, $arg_name:ident, #[$description:meta]) => {

        /// Contains
        #[$description]
        /// name.
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        // The name type *must* be a tuple struct containing exactly one field
        // so that Serde derives Deserialize and Serialize as using a simple
        // string ("") instead of an keyed object ({}).
        pub struct $type_name(String);

        impl AsRef<str> for $type_name {
            fn as_ref(&self) -> &str {
                self.0.as_ref()
            }
        }

        impl std::fmt::Display for $type_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                self.0.fmt(formatter)
            }
        }

        impl<'a> From<&'a str> for $type_name {
            fn from(s: &'a str) -> Self {
                $type_name (String::from(s))
            }
        }

        impl From<String> for $type_name {
            fn from(s: String) -> Self {
                $type_name(s)
            }
        }

        impl From<$type_name> for String {
            fn from($arg_name: $type_name) -> Self {
                $arg_name.0
            }
        }
    }
}

define_name_type!(AttachmentName, att_name, /** an attachment */);
define_name_type!(DatabaseName, db_name, /** a database */);
define_name_type!(DesignDocumentName, ddoc_name, /** a design document */);
define_name_type!(LocalDocumentName, ldoc_name, /** a local document */);
define_name_type!(NormalDocumentName, ndoc_name, /** a normal document */);
define_name_type!(ViewName, view_name, /** a view */);

#[cfg(test)]
mod name_tests {

    use {serde_json, std};

    define_name_type!(TestName, test_name, /** blah blah blah */);

    #[test]
    fn serialize_deserialize() {
        let expected = serde_json::Value::from("alpha");
        let got = serde_json::to_value(&TestName::from("alpha")).unwrap();
        assert_eq!(expected, got);
    }
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

impl DocumentId {
    #[doc(hidden)]
    pub fn new_from_string(mut s: String) -> Self {

        if s.starts_with(DESIGN_PREFIX) && s[DESIGN_PREFIX.len()..].starts_with('/') {
            let name_index = DESIGN_PREFIX.len() + '/'.len_utf8();
            s.drain(..name_index);
            DocumentId::Design(DesignDocumentName::from(s))
        } else if s.starts_with(LOCAL_PREFIX) && s[LOCAL_PREFIX.len()..].starts_with('/') {
            let name_index = LOCAL_PREFIX.len() + '/'.len_utf8();
            s.drain(..name_index);
            DocumentId::Local(LocalDocumentName::from(s))
        } else {
            DocumentId::Normal(NormalDocumentName::from(s))
        }
    }

    #[doc(hidden)]
    pub fn prefix(&self) -> Option<&'static str> {
        match self {
            &DocumentId::Normal(..) => None,
            &DocumentId::Design(..) => Some(DESIGN_PREFIX),
            &DocumentId::Local(..) => Some(LOCAL_PREFIX),
        }
    }

    #[doc(hidden)]
    pub fn name_as_str(&self) -> &str {
        match self {
            &DocumentId::Normal(ref x) => x.0.as_ref(),
            &DocumentId::Design(ref x) => x.0.as_ref(),
            &DocumentId::Local(ref x) => x.0.as_ref(),
        }
    }

    fn percent_encoded(&self) -> String {
        let name_part = percent_encode(self.name_as_str());
        match self.prefix() {
            None => format!("{}", name_part),
            Some(prefix) => format!("{}/{}", prefix, name_part),
        }
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self.prefix() {
            None => self.name_as_str().fmt(formatter),
            Some(prefix) => write!(formatter, "{}/{}", prefix, self.name_as_str()),
        }
    }
}

impl<'a> From<&'a str> for DocumentId {
    fn from(s: &'a str) -> Self {
        DocumentId::new_from_string(String::from(s))
    }
}

impl From<String> for DocumentId {
    fn from(s: String) -> Self {
        DocumentId::new_from_string(s)
    }
}

impl From<NormalDocumentName> for DocumentId {
    fn from(ndoc_name: NormalDocumentName) -> Self {
        DocumentId::Normal(ndoc_name)
    }
}


impl From<DesignDocumentName> for DocumentId {
    fn from(ddoc_name: DesignDocumentName) -> Self {
        DocumentId::Design(ddoc_name)
    }
}

impl From<LocalDocumentName> for DocumentId {
    fn from(ldoc_name: LocalDocumentName) -> Self {
        DocumentId::Local(ldoc_name)
    }
}

#[doc(hidden)]
impl serde::Serialize for DocumentId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

#[doc(hidden)]
impl<'de> serde::Deserialize<'de> for DocumentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = DocumentId;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                write!(f, "a string specifying a document id")
            }

            fn visit_str<E>(self, encoded: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(DocumentId::from(encoded))
            }

            fn visit_borrowed_str<E>(self, encoded: &'de str) -> Result<Self::Value, E>
            where
                E: std::error::Error,
            {
                Ok(DocumentId::from(encoded))
            }

            fn visit_string<E>(self, encoded: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(DocumentId::from(encoded))
            }
        }

        deserializer.deserialize_string(Visitor)
    }
}

#[cfg(test)]
mod document_id_tests {

    use super::*;
    use serde_json;

    #[test]
    fn from_normal() {
        let expected = DocumentId::Normal(NormalDocumentName::from("alpha"));
        let got = DocumentId::from("alpha");
        assert_eq!(expected, got);
    }

    #[test]
    fn from_design() {
        let expected = DocumentId::Design(DesignDocumentName::from("alpha"));
        let got = DocumentId::from("_design/alpha");
        assert_eq!(expected, got);
    }

    #[test]
    fn from_local() {
        let expected = DocumentId::Local(LocalDocumentName::from("alpha"));
        let got = DocumentId::from("_local/alpha");
        assert_eq!(expected, got);
    }

    #[test]
    fn prefix_normal() {
        assert_eq!(None, DocumentId::from("alpha").prefix());
    }

    #[test]
    fn prefix_design() {
        assert_eq!(Some("_design"), DocumentId::from("_design/alpha").prefix());
    }

    #[test]
    fn prefix_local() {
        assert_eq!(Some("_local"), DocumentId::from("_local/alpha").prefix());
    }

    #[test]
    fn display_normal() {
        let expected = "alpha";
        let got = format!("{}", DocumentId::from("alpha"));
        assert_eq!(expected, got);
    }

    #[test]
    fn display_design() {
        let expected = "_design/alpha";
        let got = format!("{}", DocumentId::from("_design/alpha"));
        assert_eq!(expected, got);
    }

    #[test]
    fn display_local() {
        let expected = "_local/alpha";
        let got = format!("{}", DocumentId::from("_local/alpha"));
        assert_eq!(expected, got);
    }

    #[test]
    fn serialize_normal() {
        let expected = serde_json::Value::String("alpha".into());
        let got = serde_json::to_value(&DocumentId::from("alpha")).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn serialize_design() {
        let expected = serde_json::Value::String("_design/alpha".into());
        let got = serde_json::to_value(&DocumentId::from("_design/alpha")).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn serialize_local() {
        let expected = serde_json::Value::String("_local/alpha".into());
        let got = serde_json::to_value(&DocumentId::from("_local/alpha")).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialize_normal() {
        let json = serde_json::Value::String("alpha".into());
        let expected = DocumentId::from("alpha");
        let got = serde_json::from_value(json).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialize_design() {
        let json = serde_json::Value::String("_design/alpha".into());
        let expected = DocumentId::from("_design/alpha");
        let got = serde_json::from_value(json).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialize_local() {
        let json = serde_json::Value::String("_local/alpha".into());
        let expected = DocumentId::from("_local/alpha");
        let got = serde_json::from_value(json).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn percent_encoded_normal() {
        let expected = "alpha%2F%25%20bravo";
        let got = DocumentId::from("alpha/% bravo").percent_encoded();
        assert_eq!(expected, got);
    }

    #[test]
    fn percent_encoded_design() {
        let expected = "_design/alpha%2F%25%20bravo";
        let got = DocumentId::from("_design/alpha/% bravo").percent_encoded();
        assert_eq!(expected, got);
    }

    #[test]
    fn percent_encoded_local() {
        let expected = "_local/alpha%2F%25%20bravo";
        let got = DocumentId::from("_local/alpha/% bravo").percent_encoded();
        assert_eq!(expected, got);
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatabasePath {
    db_name: DatabaseName,
}

impl DatabasePath {
    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DatabasePathIter {
        DatabasePathIter::DatabaseName(self)
    }
}

impl std::fmt::Display for DatabasePath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(formatter, "/{}", percent_encode(self.db_name.as_ref()))
    }
}

impl<'a, T> From<T> for DatabasePath
where
    T: Into<DatabaseName>,
{
    fn from(db_name: T) -> Self {
        DatabasePath { db_name: db_name.into() }
    }
}

#[doc(hidden)]
pub enum DatabasePathIter<'a> {
    DatabaseName(&'a DatabasePath),
    Done,
}

impl<'a> Iterator for DatabasePathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {

        let (next, item) = match self {
            &mut DatabasePathIter::DatabaseName(db_path) => (DatabasePathIter::Done, db_path.db_name.as_ref()),
            &mut DatabasePathIter::Done => {
                return None;
            }
        };

        *self = next;

        Some(item)
    }
}

#[cfg(test)]
mod database_path_tests {

    use super::*;

    #[test]
    fn iter() {
        let db_path = DatabasePath { db_name: DatabaseName::from("alpha") };
        let expected = vec!["alpha"];
        assert_eq!(expected, db_path.iter().collect::<Vec<_>>());
    }

    #[test]
    fn display() {
        let expected = "/alpha%2F%25%20bravo";
        let got = format!(
            "{}",
            DatabasePath::from(DatabaseName::from("alpha/% bravo"))
        );
        assert_eq!(expected, got);
    }
}

pub trait IntoDatabasePath {
    fn into_database_path(self) -> Result<DatabasePath, Error>;
}

impl IntoDatabasePath for &'static str {
    fn into_database_path(self) -> Result<DatabasePath, Error> {

        let mut path_extractor = try!(PathExtractor::begin(self));
        let db_name = try!(path_extractor.extract_nonempty());
        try!(path_extractor.end());

        Ok(DatabasePath { db_name: DatabaseName::from(db_name) })
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

#[cfg(test)]
mod into_database_path_tests {

    use super::*;

    #[test]
    fn static_str_ref_ok() {
        let expected = DatabasePath { db_name: DatabaseName::from("alpha") };
        let got = "/alpha".into_database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_nok() {

        use Error;

        macro_rules! nok {
            ($input:expr, $expected_error_kind:pat) => {
                match $input.into_database_path() {
                    Err(Error::PathParse{ inner: $expected_error_kind }) => (),
                    x => panic!("Got unexpected result {:?}", x),
                }
            }
        }

        nok!("", PathParseError::NoLeadingSlash);
        nok!("/", PathParseError::TooFewSegments);
        nok!("/alpha/", PathParseError::TrailingSlash);
        nok!("/alpha/bravo", PathParseError::TooManySegments);
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentPath {
    db_name: DatabaseName,
    doc_id: DocumentId,
}

impl DocumentPath {
    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.doc_id
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DocumentPathIter {
        DocumentPathIter::DatabaseName(self)
    }
}

impl std::fmt::Display for DocumentPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            formatter,
            "/{}/{}",
            percent_encode(self.db_name.as_ref()),
            self.doc_id.percent_encoded()
        )
    }
}

impl<'a, T, U> From<(T, U)> for DocumentPath
where
    T: Into<DatabasePath>,
    U: Into<DocumentId>,
{
    fn from(parts: (T, U)) -> Self {
        DocumentPath {
            db_name: parts.0.into().db_name,
            doc_id: parts.1.into(),
        }
    }
}

#[doc(hidden)]
pub enum DocumentPathIter<'a> {
    DatabaseName(&'a DocumentPath),
    DocumentPrefix(&'a DocumentPath),
    DocumentName(&'a DocumentPath),
    Done,
}

impl<'a> Iterator for DocumentPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {

        loop {
            let (next, item) = match self {
                &mut DocumentPathIter::DatabaseName(doc_path) => {
                    (
                        DocumentPathIter::DocumentPrefix(doc_path),
                        Some(doc_path.db_name.as_ref()),
                    )
                }
                &mut DocumentPathIter::DocumentPrefix(doc_path) => {
                    (
                        DocumentPathIter::DocumentName(doc_path),
                        doc_path.doc_id.prefix(),
                    )
                }
                &mut DocumentPathIter::DocumentName(doc_path) => {
                    (DocumentPathIter::Done, Some(doc_path.doc_id.name_as_str()))
                }
                &mut DocumentPathIter::Done => {
                    return None;
                }
            };

            *self = next;

            if item.is_some() {
                return item;
            }
        }
    }
}

#[cfg(test)]
mod document_path_tests {

    use super::*;

    #[test]
    fn iter_normal() {

        let doc_path = DocumentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::from("bravo"),
        };

        let expected = vec!["alpha", "bravo"];
        let got = doc_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn iter_design() {

        let doc_path = DocumentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::from("_design/bravo"),
        };

        let expected = vec!["alpha", "_design", "bravo"];
        let got = doc_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn iter_local() {

        let doc_path = DocumentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::from("_local/bravo"),
        };

        let expected = vec!["alpha", "_local", "bravo"];
        let got = doc_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn display() {
        let expected = "/alpha%2F%25%20bravo/_design/charlie%2F%25%20delta";
        let doc_path = DocumentPath::from((
            DatabaseName::from("alpha/% bravo"),
            DocumentId::from("_design/charlie/% delta"),
        ));
        let got = format!("{}", doc_path);
        assert_eq!(expected, got);
    }
}

pub trait IntoDocumentPath {
    fn into_document_path(self) -> Result<DocumentPath, Error>;
}

impl IntoDocumentPath for &'static str {
    fn into_document_path(self) -> Result<DocumentPath, Error> {

        let mut path_extractor = try!(PathExtractor::begin(self));
        let db_name = try!(path_extractor.extract_nonempty());

        let doc_id = match try!(path_extractor.extract_nonempty()) {
            x @ _ if x == DESIGN_PREFIX => {
                println!("CHECK: {:?}, {:?}", path_extractor, self);
                let doc_name = try!(path_extractor.extract_nonempty());
                DocumentId::Design(DesignDocumentName::from(doc_name))
            }
            x @ _ if x == LOCAL_PREFIX => {
                let doc_name = try!(path_extractor.extract_nonempty());
                DocumentId::Local(LocalDocumentName::from(doc_name))
            }
            x @ _ => DocumentId::Normal(NormalDocumentName::from(x)),
        };

        try!(path_extractor.end());

        Ok(DocumentPath {
            db_name: db_name.into(),
            doc_id: doc_id,
        })
    }
}

impl IntoDocumentPath for DocumentPath {
    fn into_document_path(self) -> Result<DocumentPath, Error> {
        Ok(self)
    }
}

impl<'a, T, U> IntoDocumentPath for (T, U)
where
    T: IntoDatabasePath,
    U: Into<DocumentId>,
{
    fn into_document_path(self) -> Result<DocumentPath, Error> {
        Ok(DocumentPath {
            db_name: try!(self.0.into_database_path()).db_name,
            doc_id: self.1.into(),
        })
    }
}

#[cfg(test)]
mod into_document_path_tests {

    use super::*;

    #[test]
    fn static_str_ref_ok_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::Normal(NormalDocumentName::from("bravo")),
        };
        let got = "/alpha/bravo".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_ok_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::Design(DesignDocumentName::from("bravo")),
        };
        let got = "/alpha/_design/bravo".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_ok_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::Local(LocalDocumentName::from("bravo")),
        };
        let got = "/alpha/_local/bravo".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_nok() {

        use Error;

        macro_rules! nok {
            ($input:expr, $expected_error_kind:pat) => {
                match $input.into_document_path() {
                    Err(Error::PathParse{ inner: $expected_error_kind }) => (),
                    x => panic!("Got unexpected result {:?}", x),
                }
            }
        }

        nok!("", PathParseError::NoLeadingSlash);
        nok!("/", PathParseError::TooFewSegments);
        nok!("/alpha/", PathParseError::TooFewSegments);
        nok!("/alpha/bravo/", PathParseError::TrailingSlash);
        nok!("/alpha/bravo/charlie", PathParseError::TooManySegments);
        nok!("/alpha/_design", PathParseError::TooFewSegments);
        nok!("/alpha/_design/", PathParseError::TooFewSegments);
        nok!("/alpha/_local", PathParseError::TooFewSegments);
        nok!("/alpha/_local/", PathParseError::TooFewSegments);
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DesignDocumentPath {
    db_name: DatabaseName,
    ddoc_name: DesignDocumentName,
}

impl DesignDocumentPath {
    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn design_document_name(&self) -> &DesignDocumentName {
        &self.ddoc_name
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DesignDocumentPathIter {
        DesignDocumentPathIter::DatabaseName(self)
    }
}

impl std::fmt::Display for DesignDocumentPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            formatter,
            "/{}/{}/{}",
            percent_encode(self.db_name.as_ref()),
            DESIGN_PREFIX,
            percent_encode(self.ddoc_name.as_ref())
        )
    }
}

impl<'a, T, U> From<(T, U)> for DesignDocumentPath
where
    T: Into<DatabasePath>,
    U: Into<DesignDocumentName>,
{
    fn from(parts: (T, U)) -> Self {
        DesignDocumentPath {
            db_name: parts.0.into().db_name,
            ddoc_name: parts.1.into(),
        }
    }
}

#[doc(hidden)]
pub enum DesignDocumentPathIter<'a> {
    DatabaseName(&'a DesignDocumentPath),
    DocumentPrefix(&'a DesignDocumentPath),
    DocumentName(&'a DesignDocumentPath),
    Done,
}

impl<'a> Iterator for DesignDocumentPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {

        let (next, item) = match self {
            &mut DesignDocumentPathIter::DatabaseName(doc_path) => {
                (
                    DesignDocumentPathIter::DocumentPrefix(doc_path),
                    doc_path.db_name.as_ref(),
                )
            }
            &mut DesignDocumentPathIter::DocumentPrefix(doc_path) => {
                (
                    DesignDocumentPathIter::DocumentName(doc_path),
                    DESIGN_PREFIX,
                )
            }
            &mut DesignDocumentPathIter::DocumentName(doc_path) => {
                (DesignDocumentPathIter::Done, doc_path.ddoc_name.as_ref())
            }
            &mut DesignDocumentPathIter::Done => {
                return None;
            }
        };

        *self = next;

        Some(item)
    }
}

#[cfg(test)]
mod design_document_path_tests {

    use super::*;

    #[test]
    fn iter() {

        let ddoc_path = DesignDocumentPath {
            db_name: DatabaseName::from("alpha"),
            ddoc_name: DesignDocumentName::from("bravo"),
        };

        let expected = vec!["alpha", "_design", "bravo"];
        let got = ddoc_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn display() {
        let expected = "/alpha%2F%25%20bravo/_design/charlie%2F%25%20delta";
        let ddoc_path = DesignDocumentPath::from((
            DatabaseName::from("alpha/% bravo"),
            DesignDocumentName::from("charlie/% delta"),
        ));
        let got = format!("{}", ddoc_path);
        assert_eq!(expected, got);
    }
}

pub trait IntoDesignDocumentPath {
    fn into_design_document_path(self) -> Result<DesignDocumentPath, Error>;
}

impl IntoDesignDocumentPath for &'static str {
    fn into_design_document_path(self) -> Result<DesignDocumentPath, Error> {

        let mut path_extractor = try!(PathExtractor::begin(self));
        let db_name = try!(path_extractor.extract_nonempty());
        try!(path_extractor.extract_literal(DESIGN_PREFIX));
        let ddoc_name = try!(path_extractor.extract_nonempty());
        try!(path_extractor.end());

        Ok(DesignDocumentPath {
            db_name: DatabaseName::from(db_name),
            ddoc_name: DesignDocumentName::from(ddoc_name),
        })
    }
}

impl IntoDesignDocumentPath for DesignDocumentPath {
    fn into_design_document_path(self) -> Result<DesignDocumentPath, Error> {
        Ok(self)
    }
}

impl<'a, T, U> IntoDesignDocumentPath for (T, U)
where
    T: IntoDatabasePath,
    U: Into<DesignDocumentName>,
{
    fn into_design_document_path(self) -> Result<DesignDocumentPath, Error> {
        Ok(DesignDocumentPath {
            db_name: try!(self.0.into_database_path()).db_name,
            ddoc_name: self.1.into(),
        })
    }
}

#[cfg(test)]
mod into_design_document_path_tests {

    use super::*;

    #[test]
    fn from_static_str_ref_ok() {
        let expected = DesignDocumentPath {
            db_name: DatabaseName::from("alpha"),
            ddoc_name: DesignDocumentName::from("bravo"),
        };
        let got = "/alpha/_design/bravo".into_design_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_nok() {

        use Error;
        use super::DESIGN_PREFIX;

        macro_rules! nok {
            ($input:expr, $expected_error_kind:pat) => {
                match $input.into_design_document_path() {
                    Err(Error::PathParse{ inner: $expected_error_kind }) => (),
                    x => panic!("Got unexpected result {:?}", x),
                }
            }
        }

        nok!("", PathParseError::NoLeadingSlash);
        nok!("alpha/_design/bravo", PathParseError::NoLeadingSlash);
        nok!("//alpha/_design/bravo", PathParseError::EmptySegment);
        nok!(
            "/alpha//_design/bravo",
            PathParseError::BadSegment(DESIGN_PREFIX)
        );
        nok!("/alpha/_design//bravo", PathParseError::EmptySegment);
        nok!("/alpha/_design/bravo/", PathParseError::TrailingSlash);
        nok!("/", PathParseError::TooFewSegments);
        nok!("/alpha", PathParseError::TooFewSegments);
        nok!("/alpha/_local", PathParseError::BadSegment(DESIGN_PREFIX));
        nok!("/alpha/_design", PathParseError::TooFewSegments);
        nok!("/alpha/_design/", PathParseError::TooFewSegments);
        nok!(
            "/alpha/_design/bravo/charlie",
            PathParseError::TooManySegments
        );
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AttachmentPath {
    db_name: DatabaseName,
    doc_id: DocumentId,
    att_name: AttachmentName,
}

impl AttachmentPath {
    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.doc_id
    }

    pub fn attachment_name(&self) -> &AttachmentName {
        &self.att_name
    }

    #[doc(hidden)]
    pub fn iter(&self) -> AttachmentPathIter {
        AttachmentPathIter::DatabaseName(self)
    }
}

impl std::fmt::Display for AttachmentPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            formatter,
            "/{}/{}/{}",
            percent_encode(self.db_name.as_ref()),
            self.doc_id.percent_encoded(),
            percent_encode(self.att_name.as_ref())
        )
    }
}

impl<'a, T, U> From<(T, U)> for AttachmentPath
where
    T: Into<DocumentPath>,
    U: Into<AttachmentName>,
{
    fn from(parts: (T, U)) -> Self {
        let doc_path = parts.0.into();
        AttachmentPath {
            db_name: doc_path.db_name,
            doc_id: doc_path.doc_id,
            att_name: parts.1.into(),
        }
    }
}

impl<'a, T, U, V> From<(T, U, V)> for AttachmentPath
where
    T: Into<DatabasePath>,
    U: Into<DocumentId>,
    V: Into<AttachmentName>,
{
    fn from(parts: (T, U, V)) -> Self {
        AttachmentPath {
            db_name: parts.0.into().db_name,
            doc_id: parts.1.into(),
            att_name: parts.2.into(),
        }
    }
}

#[doc(hidden)]
pub enum AttachmentPathIter<'a> {
    DatabaseName(&'a AttachmentPath),
    DocumentPrefix(&'a AttachmentPath),
    DocumentName(&'a AttachmentPath),
    AttachmentName(&'a AttachmentPath),
    Done,
}

impl<'a> Iterator for AttachmentPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {

        loop {
            let (next, item) = match self {
                &mut AttachmentPathIter::DatabaseName(att_path) => {
                    (
                        AttachmentPathIter::DocumentPrefix(att_path),
                        Some(att_path.db_name.as_ref()),
                    )
                }
                &mut AttachmentPathIter::DocumentPrefix(att_path) => {
                    (
                        AttachmentPathIter::DocumentName(att_path),
                        att_path.doc_id.prefix(),
                    )
                }
                &mut AttachmentPathIter::DocumentName(att_path) => {
                    (
                        AttachmentPathIter::AttachmentName(att_path),
                        Some(att_path.doc_id.name_as_str()),
                    )
                }
                &mut AttachmentPathIter::AttachmentName(att_path) => {
                    (AttachmentPathIter::Done, Some(att_path.att_name.as_ref()))
                }
                &mut AttachmentPathIter::Done => {
                    return None;
                }
            };

            *self = next;

            if item.is_some() {
                return item;
            }
        }
    }
}

#[cfg(test)]
mod attachment_path_tests {

    use super::*;

    #[test]
    fn iter_normal() {

        let att_path = AttachmentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::Normal(NormalDocumentName::from("bravo")),
            att_name: AttachmentName::from("charlie"),
        };

        let expected = vec!["alpha", "bravo", "charlie"];
        let got = att_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn iter_design() {

        let att_path = AttachmentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::Design(DesignDocumentName::from("bravo")),
            att_name: AttachmentName::from("charlie"),
        };

        let expected = vec!["alpha", "_design", "bravo", "charlie"];
        let got = att_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn iter_local() {

        let att_path = AttachmentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::Local(LocalDocumentName::from("bravo")),
            att_name: AttachmentName::from("charlie"),
        };

        let expected = vec!["alpha", "_local", "bravo", "charlie"];
        let got = att_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn display() {
        let expected = "/alpha%2F%25%20bravo/_design/charlie%2F%25%20delta/echo%2F%25%20foxtrot";
        let att_path = AttachmentPath::from((
            DatabaseName::from("alpha/% bravo"),
            DocumentId::from("_design/charlie/% delta"),
            AttachmentName::from("echo/% foxtrot"),
        ));
        let got = format!("{}", att_path);
        assert_eq!(expected, got);
    }
}

pub trait IntoAttachmentPath {
    fn into_attachment_path(self) -> Result<AttachmentPath, Error>;
}

impl IntoAttachmentPath for &'static str {
    fn into_attachment_path(self) -> Result<AttachmentPath, Error> {

        let mut path_extractor = try!(PathExtractor::begin(self));
        let db_name = try!(path_extractor.extract_nonempty());

        let doc_id = match try!(path_extractor.extract_nonempty()) {
            x @ _ if x == DESIGN_PREFIX => {
                println!("CHECK: {:?}, {:?}", path_extractor, self);
                let doc_name = try!(path_extractor.extract_nonempty());
                DocumentId::Design(DesignDocumentName::from(doc_name))
            }
            x @ _ if x == LOCAL_PREFIX => {
                let doc_name = try!(path_extractor.extract_nonempty());
                DocumentId::Local(LocalDocumentName::from(doc_name))
            }
            x @ _ => DocumentId::Normal(NormalDocumentName::from(x)),
        };

        let att_name = try!(path_extractor.extract_nonempty());
        try!(path_extractor.end());

        Ok(AttachmentPath {
            db_name: DatabaseName::from(db_name),
            doc_id: doc_id,
            att_name: AttachmentName::from(att_name),
        })
    }
}

impl IntoAttachmentPath for AttachmentPath {
    fn into_attachment_path(self) -> Result<AttachmentPath, Error> {
        Ok(self)
    }
}

impl<'a, T, U> IntoAttachmentPath for (T, U)
where
    T: IntoDocumentPath,
    U: Into<AttachmentName>,
{
    fn into_attachment_path(self) -> Result<AttachmentPath, Error> {
        let doc_path = try!(self.0.into_document_path());
        Ok(AttachmentPath {
            db_name: doc_path.db_name,
            doc_id: doc_path.doc_id,
            att_name: self.1.into(),
        })
    }
}

impl<'a, T, U, V> IntoAttachmentPath for (T, U, V)
where
    T: IntoDatabasePath,
    U: Into<DocumentId>,
    V: Into<AttachmentName>,
{
    fn into_attachment_path(self) -> Result<AttachmentPath, Error> {
        Ok(AttachmentPath {
            db_name: try!(self.0.into_database_path()).db_name,
            doc_id: self.1.into(),
            att_name: self.2.into(),
        })
    }
}

#[cfg(test)]
mod into_attachment_path_tests {

    use super::*;

    #[test]
    fn static_str_ref_ok_normal() {
        let expected = AttachmentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::Normal(NormalDocumentName::from("bravo")),
            att_name: AttachmentName::from("charlie"),
        };
        let got = "/alpha/bravo/charlie".into_attachment_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_ok_design() {
        let expected = AttachmentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::Design(DesignDocumentName::from("bravo")),
            att_name: AttachmentName::from("charlie"),
        };
        let got = "/alpha/_design/bravo/charlie"
            .into_attachment_path()
            .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_ok_local() {
        let expected = AttachmentPath {
            db_name: DatabaseName::from("alpha"),
            doc_id: DocumentId::Local(LocalDocumentName::from("bravo")),
            att_name: AttachmentName::from("charlie"),
        };
        let got = "/alpha/_local/bravo/charlie"
            .into_attachment_path()
            .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_nok() {

        use Error;

        macro_rules! nok {
            ($input:expr, $expected_error_kind:pat) => {
                match $input.into_attachment_path() {
                    Err(Error::PathParse{ inner: $expected_error_kind }) => (),
                    x => panic!("Got unexpected result {:?}", x),
                }
            }
        }

        nok!("", PathParseError::NoLeadingSlash);
        nok!("/", PathParseError::TooFewSegments);
        nok!("/alpha/", PathParseError::TooFewSegments);
        nok!("/alpha/bravo/", PathParseError::TooFewSegments);
        nok!("/alpha/bravo/charlie/", PathParseError::TrailingSlash);
        nok!(
            "/alpha/bravo/charlie/delta",
            PathParseError::TooManySegments
        );
        nok!("/alpha/_design", PathParseError::TooFewSegments);
        nok!("/alpha/_design/", PathParseError::TooFewSegments);
        nok!("/alpha/_design/bravo", PathParseError::TooFewSegments);
        nok!("/alpha/_local", PathParseError::TooFewSegments);
        nok!("/alpha/_local/", PathParseError::TooFewSegments);
        nok!("/alpha/_local/bravo", PathParseError::TooFewSegments);
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ViewPath {
    db_name: DatabaseName,
    ddoc_name: DesignDocumentName,
    view_name: ViewName,
}

impl ViewPath {
    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn design_document_name(&self) -> &DesignDocumentName {
        &self.ddoc_name
    }

    pub fn attachment_name(&self) -> &ViewName {
        &self.view_name
    }

    #[doc(hidden)]
    pub fn iter(&self) -> ViewPathIter {
        ViewPathIter::DatabaseName(self)
    }
}

impl std::fmt::Display for ViewPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            formatter,
            "/{}/{}/{}/{}",
            percent_encode(self.db_name.as_ref()),
            DESIGN_PREFIX,
            percent_encode(self.ddoc_name.as_ref()),
            percent_encode(self.view_name.as_ref())
        )
    }
}

impl<'a, T, U> From<(T, U)> for ViewPath
where
    T: Into<DesignDocumentPath>,
    U: Into<ViewName>,
{
    fn from(parts: (T, U)) -> Self {
        let ddoc_path = parts.0.into();
        ViewPath {
            db_name: ddoc_path.db_name,
            ddoc_name: ddoc_path.ddoc_name,
            view_name: parts.1.into(),
        }
    }
}

impl<'a, T, U, V> From<(T, U, V)> for ViewPath
where
    T: Into<DatabasePath>,
    U: Into<DesignDocumentName>,
    V: Into<ViewName>,
{
    fn from(parts: (T, U, V)) -> Self {
        ViewPath {
            db_name: parts.0.into().db_name,
            ddoc_name: parts.1.into(),
            view_name: parts.2.into(),
        }
    }
}

#[doc(hidden)]
pub enum ViewPathIter<'a> {
    DatabaseName(&'a ViewPath),
    DocumentPrefix(&'a ViewPath),
    DocumentName(&'a ViewPath),
    ViewPrefix(&'a ViewPath),
    ViewName(&'a ViewPath),
    Done,
}

impl<'a> Iterator for ViewPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {

        let (next, item) = match self {
            &mut ViewPathIter::DatabaseName(view_path) => {
                (
                    ViewPathIter::DocumentPrefix(view_path),
                    view_path.db_name.as_ref(),
                )
            }
            &mut ViewPathIter::DocumentPrefix(view_path) => (ViewPathIter::DocumentName(view_path), DESIGN_PREFIX),
            &mut ViewPathIter::DocumentName(view_path) => {
                (
                    ViewPathIter::ViewPrefix(view_path),
                    view_path.ddoc_name.as_ref(),
                )
            }
            &mut ViewPathIter::ViewPrefix(view_path) => (ViewPathIter::ViewName(view_path), VIEW_PREFIX),
            &mut ViewPathIter::ViewName(view_path) => (ViewPathIter::Done, view_path.view_name.as_ref()),
            &mut ViewPathIter::Done => {
                return None;
            }
        };

        *self = next;

        Some(item)
    }
}

#[cfg(test)]
mod view_path_tests {

    use super::*;

    #[test]
    fn iter() {

        let view_path = ViewPath {
            db_name: DatabaseName::from("alpha"),
            ddoc_name: DesignDocumentName::from("bravo"),
            view_name: ViewName::from("charlie"),
        };

        let expected = vec!["alpha", "_design", "bravo", "_view", "charlie"];
        let got = view_path.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn display() {
        let expected = "/alpha%2F%25%20bravo/_design/charlie%2F%25%20delta/echo%2F%25%20foxtrot";
        let view_path = ViewPath::from((
            DatabaseName::from("alpha/% bravo"),
            DesignDocumentName::from("charlie/% delta"),
            ViewName::from("echo/% foxtrot"),
        ));
        let got = format!("{}", view_path);
        assert_eq!(expected, got);
    }
}

pub trait IntoViewPath {
    fn into_view_path(self) -> Result<ViewPath, Error>;
}

impl IntoViewPath for &'static str {
    fn into_view_path(self) -> Result<ViewPath, Error> {

        let mut path_extractor = try!(PathExtractor::begin(self));
        let db_name = try!(path_extractor.extract_nonempty());
        try!(path_extractor.extract_literal(DESIGN_PREFIX));
        let ddoc_name = try!(path_extractor.extract_nonempty());
        try!(path_extractor.extract_literal(VIEW_PREFIX));
        let view_name = try!(path_extractor.extract_nonempty());
        try!(path_extractor.end());

        Ok(ViewPath {
            db_name: DatabaseName::from(db_name),
            ddoc_name: DesignDocumentName::from(ddoc_name),
            view_name: ViewName::from(view_name),
        })
    }
}

impl IntoViewPath for ViewPath {
    fn into_view_path(self) -> Result<ViewPath, Error> {
        Ok(self)
    }
}

impl<'a, T, U> IntoViewPath for (T, U)
where
    T: IntoDesignDocumentPath,
    U: Into<ViewName>,
{
    fn into_view_path(self) -> Result<ViewPath, Error> {
        let ddoc_path = try!(self.0.into_design_document_path());
        Ok(ViewPath {
            db_name: ddoc_path.db_name,
            ddoc_name: ddoc_path.ddoc_name,
            view_name: self.1.into(),
        })
    }
}

impl<'a, T, U, V> IntoViewPath for (T, U, V)
where
    T: IntoDatabasePath,
    U: Into<DesignDocumentName>,
    V: Into<ViewName>,
{
    fn into_view_path(self) -> Result<ViewPath, Error> {
        Ok(ViewPath {
            db_name: try!(self.0.into_database_path()).db_name,
            ddoc_name: self.1.into(),
            view_name: self.2.into(),
        })
    }
}

#[cfg(test)]
mod into_view_path_tests {

    use super::*;

    #[test]
    fn static_str_ref_ok() {
        let expected = ViewPath {
            db_name: DatabaseName::from("alpha"),
            ddoc_name: DesignDocumentName::from("bravo"),
            view_name: ViewName::from("charlie"),
        };
        let got = "/alpha/_design/bravo/_view/charlie"
            .into_view_path()
            .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_nok() {

        use Error;
        use super::{DESIGN_PREFIX, VIEW_PREFIX};

        macro_rules! nok {
            ($input:expr, $expected_error_kind:pat) => {
                match $input.into_view_path() {
                    Err(Error::PathParse{ inner: $expected_error_kind }) => (),
                    x => panic!("Got unexpected result {:?}", x),
                }
            }
        }

        nok!("", PathParseError::NoLeadingSlash);
        nok!("/", PathParseError::TooFewSegments);
        nok!("/alpha", PathParseError::TooFewSegments);
        nok!("/alpha/", PathParseError::TooFewSegments);
        nok!("/alpha/_design", PathParseError::TooFewSegments);
        nok!("/alpha/_design/", PathParseError::TooFewSegments);
        nok!("/alpha/_design/bravo", PathParseError::TooFewSegments);
        nok!("/alpha/_design/bravo/", PathParseError::TooFewSegments);
        nok!("/alpha/_design/bravo/_view", PathParseError::TooFewSegments);
        nok!(
            "/alpha/_design/bravo/_view/",
            PathParseError::TooFewSegments
        );
        nok!(
            "/alpha/_design/bravo/_view/charlie/",
            PathParseError::TrailingSlash
        );
        nok!(
            "/alpha/_design/bravo/_view/charlie/delta",
            PathParseError::TooManySegments
        );
        nok!(
            "/alpha/_local/bravo/_view/charlie/",
            PathParseError::BadSegment(DESIGN_PREFIX)
        );
        nok!(
            "/alpha/bravo/_view/charlie/",
            PathParseError::BadSegment(DESIGN_PREFIX)
        );
        nok!(
            "/alpha/_design/bravo/invalid/charlie",
            PathParseError::BadSegment(VIEW_PREFIX)
        );
        nok!(
            "/alpha/_design/bravo/charlie",
            PathParseError::BadSegment(VIEW_PREFIX)
        );
    }
}
