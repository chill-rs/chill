use serde;
use std;

// FIXME: Make AttachmentName a unique type.
pub type AttachmentName = String;

// FIXME: Make DatabaseName a unique type.
pub type DatabaseName = String;

// FIXME: Make DesignDocumentName a unique type.
pub type DesignDocumentName = String;

// FIXME: Make LocalDocumentName a unique type.
pub type LocalDocumentName = String;

// FIXME: Make NormalDocumentName a unique type.
pub type NormalDocumentName = String;

/// A _document id_ uniquely identifies a document within a database.
///
/// A **document id** pairs a document type and name. For example, given the
/// HTTP request to `GET http://example.com:5984/db/_design/design-doc`, the
/// document id comprises `_design/design-doc` and specifies a design document
/// with the name `design-doc`. Combined with a **database name**, a document id
/// uniquely identifies a document on a CouchDB server.
///
/// There are three types of documents: **design** (i.e., starts with
/// `_design/`), **local** (i.e., starts with `_local/`), and **normal** (i.e.,
/// all other documents). Each type is expressed as an enum variant that owns
/// the underlying document name.
///
/// Although the `DocumentId` type implements the `Ord` and `PartialOrd` traits,
/// Chill provides no guarantee how that ordering is defined and may change the
/// definition between any two releases of the crate. That is, for any two
/// `DocumentId` values `a` and `b`, the expression `a < b` may hold true now
/// but not in a subsequent release. Consequently, applications must not rely
/// upon any particular ordering definition. Chill implements ordering so that
/// applications may use the `DocumentId` type as a key in an associative
/// collection.
///
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DocumentId {
    /// Design document (i.e., starts with `_design/`).
    Design(DesignDocumentName),

    /// Local document (i.e., starts with `_local/`).
    Local(LocalDocumentName),

    /// Normal document—i.e., neither a design document nor a local document.
    Normal(NormalDocumentName),
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        use DocumentId::*;
        match self {
            &Normal(ref name) => write!(f, "{}", name),
            &Design(ref name) => write!(f, "_design/{}", name),
            &Local(ref name) => write!(f, "_local/{}", name),
        }
    }
}

impl<'a> From<&'a str> for DocumentId {
    fn from(name: &str) -> Self {
        DocumentId::from(name.to_owned())
    }
}

impl From<String> for DocumentId {
    fn from(name: String) -> Self {
        let design = "_design/";
        let local = "_local/";
        if name.starts_with(design) {
            let name = name[design.len()..].to_owned();
            DocumentId::Design(name.into())
        } else if name.starts_with(local) {
            let name = name[local.len()..].to_owned();
            DocumentId::Local(name.into())
        } else {
            DocumentId::Normal(name.into())
        }
    }
}

impl From<DocumentId> for String {
    fn from(doc_id: DocumentId) -> Self {
        use DocumentId::*;
        match doc_id {
            Normal(name) => name.into(),
            Design(name) => format!("_design/{}", name),
            Local(name) => format!("_local/{}", name),
        }
    }
}

impl serde::Serialize for DocumentId {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        match self {
            &DocumentId::Normal(ref name) => serializer.serialize_str(name.as_ref()),
            _ => serializer.serialize_str(self.to_string().as_ref()),
        }
    }
}

impl serde::Deserialize for DocumentId {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = DocumentId;

            fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                Ok(DocumentId::from(value))
            }

            fn visit_string<E>(&mut self, value: String) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                Ok(DocumentId::from(value))
            }
        }

        deserializer.deserialize(Visitor)
    }
}

#[cfg(test)]
mod tests {

    use serde_json;
    use super::DocumentId;

    #[test]
    fn document_id_display_design() {
        let expected = "_design/foo";
        let got = format!("{}", DocumentId::Design("foo".into()));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_display_local() {
        let expected = "_local/foo";
        let got = format!("{}", DocumentId::Local("foo".into()));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_display_normal() {
        let expected = "foo";
        let got = format!("{}", DocumentId::Normal("foo".into()));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_str_ref_design() {
        let expected = DocumentId::Design("foo".into());
        let got = DocumentId::from("_design/foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_str_ref_local() {
        let expected = DocumentId::Local("foo".into());
        let got = DocumentId::from("_local/foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_str_ref_normal() {
        let expected = DocumentId::Normal("foo".into());
        let got = DocumentId::from("foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_string_design() {
        let expected = DocumentId::Design("foo".into());
        let got = DocumentId::from("_design/foo".to_owned());
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_string_local() {
        let expected = DocumentId::Local("foo".into());
        let got = DocumentId::from("_local/foo".to_owned());
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_string_normal() {
        let expected = DocumentId::Normal("foo".into());
        let got = DocumentId::from("foo".to_owned());
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_string_from_design() {
        let expected = "_design/foo".to_owned();
        let got = String::from(DocumentId::Design("foo".into()));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_string_from_local() {
        let expected = "_local/foo".to_owned();
        let got = String::from(DocumentId::Local("foo".into()));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_string_from_normal() {
        let expected = "foo".to_owned();
        let got = String::from(DocumentId::Normal("foo".into()));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_serialize_ok_design() {
        let expected = serde_json::Value::String("_design/foo".to_owned());
        let source = DocumentId::Design("foo".into());
        let s = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&s).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_serialize_ok_local() {
        let expected = serde_json::Value::String("_local/foo".to_owned());
        let source = DocumentId::Local("foo".into());
        let s = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&s).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_serialize_ok_normal() {
        let expected = serde_json::Value::String("foo".to_owned());
        let source = DocumentId::Normal("foo".into());
        let s = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&s).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_deserialize_ok_design() {
        let expected = DocumentId::Design("foo".into());
        let source = serde_json::Value::String("_design/foo".to_owned());
        let s = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&s).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_deserialize_ok_local() {
        let expected = DocumentId::Local("foo".into());
        let source = serde_json::Value::String("_local/foo".to_owned());
        let s = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&s).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_deserialize_ok_normal() {
        let expected = DocumentId::Normal("foo".into());
        let source = serde_json::Value::String("foo".to_owned());
        let s = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&s).unwrap();
        assert_eq!(expected, got);
    }
}
