use serde;
use std;
use super::*;

impl<'a> DocumentId<'a> {
    fn design_prefix() -> &'static str {
        "_design"
    }

    fn local_prefix() -> &'static str {
        "_local"
    }

    #[doc(hidden)]
    pub fn has_prefix(&self) -> bool {
        self.prefix_as_str().is_some()
    }

    #[doc(hidden)]
    pub fn prefix_as_str(&self) -> Option<&'static str> {
        match self {
            &DocumentId::Normal(_) => None,
            &DocumentId::Design(_) => Some(DocumentId::design_prefix()),
            &DocumentId::Local(_) => Some(DocumentId::local_prefix()),
        }
    }

    #[doc(hidden)]
    pub fn name_as_str(&self) -> &'a str {
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

impl<'a> From<&'a str> for DocumentId<'a> {
    fn from(s: &'a str) -> Self {

        let design_prefix = DocumentId::design_prefix();
        let local_prefix = DocumentId::local_prefix();

        if s.starts_with(design_prefix) && s[design_prefix.len()..].starts_with('/') {
            DocumentId::Design(DesignDocumentName::new(&s[design_prefix.len() + 1..]))
        } else if s.starts_with(local_prefix) && s[local_prefix.len()..].starts_with('/') {
            DocumentId::Local(LocalDocumentName::new(&s[local_prefix.len() + 1..]))
        } else {
            DocumentId::Normal(NormalDocumentName::new(s))
        }
    }
}

impl<'a> From<&'a DocumentIdBuf> for DocumentId<'a> {
    fn from(doc_id: &'a DocumentIdBuf) -> Self {
        match doc_id {
            &DocumentIdBuf::Normal(ref doc_name_buf) => DocumentId::Normal(&doc_name_buf),
            &DocumentIdBuf::Design(ref doc_name_buf) => DocumentId::Design(&doc_name_buf),
            &DocumentIdBuf::Local(ref doc_name_buf) => DocumentId::Local(&doc_name_buf),
        }
    }
}

impl<'a> From<&'a NormalDocumentName> for DocumentId<'a> {
    fn from(doc_name: &'a NormalDocumentName) -> Self {
        DocumentId::Normal(doc_name)
    }
}

impl<'a> From<&'a NormalDocumentNameBuf> for DocumentId<'a> {
    fn from(doc_name: &'a NormalDocumentNameBuf) -> Self {
        DocumentId::Normal(doc_name)
    }
}

impl<'a> From<&'a DesignDocumentName> for DocumentId<'a> {
    fn from(doc_name: &'a DesignDocumentName) -> Self {
        DocumentId::Design(doc_name)
    }
}

impl<'a> From<&'a DesignDocumentNameBuf> for DocumentId<'a> {
    fn from(doc_name: &'a DesignDocumentNameBuf) -> Self {
        DocumentId::Design(doc_name)
    }
}

impl<'a> From<&'a LocalDocumentName> for DocumentId<'a> {
    fn from(doc_name: &'a LocalDocumentName) -> Self {
        DocumentId::Local(doc_name)
    }
}

impl<'a> From<&'a LocalDocumentNameBuf> for DocumentId<'a> {
    fn from(doc_name: &'a LocalDocumentNameBuf) -> Self {
        DocumentId::Local(doc_name)
    }
}

impl<'a> serde::Serialize for DocumentId<'a> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        self.to_string().serialize(serializer)
    }
}

impl DocumentIdBuf {
    #[doc(hidden)]
    pub fn as_document_id(&self) -> DocumentId {
        match self {
            &DocumentIdBuf::Normal(ref doc_name_buf) => DocumentId::Normal(&doc_name_buf),
            &DocumentIdBuf::Design(ref doc_name_buf) => DocumentId::Design(&doc_name_buf),
            &DocumentIdBuf::Local(ref doc_name_buf) => DocumentId::Local(&doc_name_buf),
        }
    }
}

impl std::fmt::Display for DocumentIdBuf {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        DocumentId::from(self).fmt(formatter)
    }
}

impl<'a> From<&'a str> for DocumentIdBuf {
    fn from(s: &'a str) -> Self {
        match DocumentId::from(s) {
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

#[cfg(test)]
mod tests {

    use serde_json;
    use super::super::*;

    #[test]
    fn document_id_has_prefix_normal() {
        let doc_id = DocumentId::Normal(NormalDocumentName::new("foo"));
        assert_eq!(false, doc_id.has_prefix());
    }

    #[test]
    fn document_id_has_prefix_design() {
        let doc_id = DocumentId::Design(DesignDocumentName::new("foo"));
        assert_eq!(true, doc_id.has_prefix());
    }

    #[test]
    fn document_id_has_prefix_local() {
        let doc_id = DocumentId::Local(LocalDocumentName::new("foo"));
        assert_eq!(true, doc_id.has_prefix());
    }

    #[test]
    fn document_id_prefix_as_str_normal() {
        let doc_id = DocumentId::Normal(NormalDocumentName::new("foo"));
        assert_eq!(None, doc_id.prefix_as_str());
    }

    #[test]
    fn document_id_prefix_as_str_design() {
        let doc_id = DocumentId::Design(DesignDocumentName::new("foo"));
        assert_eq!(Some("_design"), doc_id.prefix_as_str());
    }

    #[test]
    fn document_id_prefix_as_str_local() {
        let doc_id = DocumentId::Local(LocalDocumentName::new("foo"));
        assert_eq!(Some("_local"), doc_id.prefix_as_str());
    }

    #[test]
    fn document_id_name_as_str_normal() {
        let doc_id = DocumentId::Normal(NormalDocumentName::new("foo"));
        assert_eq!("foo", doc_id.name_as_str());
    }

    #[test]
    fn document_id_name_as_str_design() {
        let doc_id = DocumentId::Design(DesignDocumentName::new("foo"));
        assert_eq!("foo", doc_id.name_as_str());
    }

    #[test]
    fn document_id_name_as_str_local() {
        let doc_id = DocumentId::Local(LocalDocumentName::new("foo"));
        assert_eq!("foo", doc_id.name_as_str());
    }

    #[test]
    fn document_id_display_normal() {
        let doc_id = DocumentId::Normal(NormalDocumentName::new("foo"));
        assert_eq!("foo", format!("{}", doc_id));
    }

    #[test]
    fn document_id_display_design() {
        let doc_id = DocumentId::Design(DesignDocumentName::new("foo"));
        assert_eq!("_design/foo", format!("{}", doc_id));
    }

    #[test]
    fn document_id_display_local() {
        let doc_id = DocumentId::Local(LocalDocumentName::new("foo"));
        assert_eq!("_local/foo", format!("{}", doc_id));
    }

    #[test]
    fn document_id_from_str_ref_normal() {
        let expected = DocumentId::Normal(NormalDocumentName::new("foo"));
        let got = DocumentId::from("foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_str_ref_normal_begins_with_design() {
        // This is an invalid document name, but our type should still exhibit
        // sane behavior.
        let expected = DocumentId::Normal(NormalDocumentName::new("_designfoo"));
        let got = DocumentId::from("_designfoo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_str_ref_normal_begins_with_local() {
        // This is an invalid document name, but our type should still exhibit
        // sane behavior.
        let expected = DocumentId::Normal(NormalDocumentName::new("_localfoo"));
        let got = DocumentId::from("_localfoo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_str_ref_design() {
        let expected = DocumentId::Design(DesignDocumentName::new("foo"));
        let got = DocumentId::from("_design/foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_str_ref_local() {
        let expected = DocumentId::Local(LocalDocumentName::new("foo"));
        let got = DocumentId::from("_local/foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_from_document_id() {
        let doc_id = DocumentId::Normal(NormalDocumentName::new("foo"));
        assert_eq!(doc_id, DocumentId::from(doc_id.clone()));
    }

    #[test]
    fn document_id_from_document_id_buf_ref() {
        let expected = DocumentId::Normal(NormalDocumentName::new("foo"));
        let doc_id_buf = DocumentIdBuf::Normal(NormalDocumentNameBuf::from("foo"));
        assert_eq!(expected, DocumentId::from(&doc_id_buf));
    }

    #[test]
    fn document_id_from_normal_document_name() {
        let expected = DocumentId::Normal(NormalDocumentName::new("foo"));
        let doc_name = NormalDocumentName::new("foo");
        assert_eq!(expected, DocumentId::from(doc_name));
    }

    #[test]
    fn document_id_from_normal_document_name_buf() {
        let expected = DocumentId::Normal(NormalDocumentName::new("foo"));
        let doc_name = NormalDocumentNameBuf::from("foo");
        assert_eq!(expected, DocumentId::from(&doc_name));
    }

    #[test]
    fn document_id_from_design_document_name() {
        let expected = DocumentId::Design(DesignDocumentName::new("foo"));
        let doc_name = DesignDocumentName::new("foo");
        assert_eq!(expected, DocumentId::from(doc_name));
    }

    #[test]
    fn document_id_from_design_document_name_buf() {
        let expected = DocumentId::Design(DesignDocumentName::new("foo"));
        let doc_name = DesignDocumentNameBuf::from("foo");
        assert_eq!(expected, DocumentId::from(&doc_name));
    }

    #[test]
    fn document_id_from_local_document_name() {
        let expected = DocumentId::Local(LocalDocumentName::new("foo"));
        let doc_name = LocalDocumentName::new("foo");
        assert_eq!(expected, DocumentId::from(doc_name));
    }

    #[test]
    fn document_id_from_local_document_name_buf() {
        let expected = DocumentId::Local(LocalDocumentName::new("foo"));
        let doc_name = LocalDocumentNameBuf::from("foo");
        assert_eq!(expected, DocumentId::from(&doc_name));
    }

    #[test]
    fn document_id_serialize_normal() {
        let expected = serde_json::Value::String(String::from("foo"));
        let doc_id = DocumentId::Normal(NormalDocumentName::new("foo"));
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
        let doc_id = DocumentId::Local(LocalDocumentName::new("foo"));
        assert_eq!(expected, serde_json::to_value(&doc_id));
    }

    #[test]
    fn document_id_buf_as_document_id_normal() {
        let doc_id_buf = DocumentIdBuf::Normal(NormalDocumentNameBuf::from("foo"));
        let expected = DocumentId::Normal(NormalDocumentName::new("foo"));
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
        let doc_id_buf = DocumentIdBuf::Local(LocalDocumentNameBuf::from("foo"));
        let expected = DocumentId::Local(LocalDocumentName::new("foo"));
        assert_eq!(expected, doc_id_buf.as_document_id());
    }

    #[test]
    fn document_id_buf_display_normal() {
        let doc_id_buf = DocumentIdBuf::Normal(NormalDocumentNameBuf::from("foo"));
        assert_eq!("foo", format!("{}", doc_id_buf));
    }

    #[test]
    fn document_id_buf_display_design() {
        let doc_id_buf = DocumentIdBuf::Design(DesignDocumentNameBuf::from("foo"));
        assert_eq!("_design/foo", format!("{}", doc_id_buf));
    }

    #[test]
    fn document_id_buf_display_local() {
        let doc_id_buf = DocumentIdBuf::Local(LocalDocumentNameBuf::from("foo"));
        assert_eq!("_local/foo", format!("{}", doc_id_buf));
    }

    #[test]
    fn document_id_buf_from_str_ref_normal() {
        let expected = DocumentIdBuf::Normal(NormalDocumentNameBuf::from("foo"));
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
        let expected = DocumentIdBuf::Local(LocalDocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from("_local/foo");
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_from_string_normal() {
        let expected = DocumentIdBuf::Normal(NormalDocumentNameBuf::from("foo"));
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
        let expected = DocumentIdBuf::Local(LocalDocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from(String::from("_local/foo"));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_from_document_id_normal() {
        let expected = DocumentIdBuf::Normal(NormalDocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from(DocumentId::Normal(NormalDocumentName::new("foo")));
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
        let expected = DocumentIdBuf::Local(LocalDocumentNameBuf::from("foo"));
        let got = DocumentIdBuf::from(DocumentId::Local(LocalDocumentName::new("foo")));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_buf_deserialize_normal() {
        let expected = DocumentIdBuf::Normal(NormalDocumentNameBuf::from("foo"));
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
        let expected = DocumentIdBuf::Local(LocalDocumentNameBuf::from("foo"));
        let got = serde_json::from_value(serde_json::Value::String(String::from("_local/foo")))
                      .unwrap();
        assert_eq!(expected, got);
    }
}
