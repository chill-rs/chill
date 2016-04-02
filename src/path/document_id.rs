use prelude_impl::*;
use serde;
use std;

impl<'a> DocumentIdRef<'a> {}

impl<'a> DocumentIdRef<'a> {
    #[doc(hidden)]
    pub fn prefix(&self) -> Option<&'static str> {
        match self {
            &DocumentIdRef::Normal(..) => None,
            &DocumentIdRef::Design(..) => Some(DocumentId::design_prefix()),
            &DocumentIdRef::Local(..) => Some(DocumentId::local_prefix()),
        }
    }

    #[doc(hidden)]
    pub fn name_as_str(&self) -> &'a str {
        match self {
            &DocumentIdRef::Normal(x) => x.inner,
            &DocumentIdRef::Design(x) => x.inner,
            &DocumentIdRef::Local(x) => x.inner,
        }
    }
}

impl DocumentId {
    #[doc(hidden)]
    pub fn design_prefix() -> &'static str {
        "_design"
    }

    #[doc(hidden)]
    pub fn local_prefix() -> &'static str {
        "_local"
    }

    pub fn as_ref(&self) -> DocumentIdRef {
        match self {
            &DocumentId::Normal(ref x) => DocumentIdRef::Normal(x.as_ref()),
            &DocumentId::Design(ref x) => DocumentIdRef::Design(x.as_ref()),
            &DocumentId::Local(ref x) => DocumentIdRef::Local(x.as_ref()),
        }
    }
}

impl<'a> std::fmt::Display for DocumentIdRef<'a> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            &DocumentIdRef::Normal(x) => x.fmt(formatter),
            &DocumentIdRef::Design(x) => write!(formatter, "{}/{}", DocumentId::design_prefix(), x),
            &DocumentIdRef::Local(x) => write!(formatter, "{}/{}", DocumentId::local_prefix(), x),
        }
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        DocumentIdRef::from(self).fmt(formatter)
    }
}

impl<'a> From<&'a str> for DocumentIdRef<'a> {
    fn from(s: &'a str) -> Self {

        let design_prefix = DocumentId::design_prefix();
        let local_prefix = DocumentId::local_prefix();

        if s.starts_with(design_prefix) && s[design_prefix.len()..].starts_with('/') {
            DocumentIdRef::Design(DesignDocumentNameRef::new(&s[design_prefix.len() + 1..]))
        } else if s.starts_with(local_prefix) && s[local_prefix.len()..].starts_with('/') {
            DocumentIdRef::Local(LocalDocumentNameRef::new(&s[local_prefix.len() + 1..]))
        } else {
            DocumentIdRef::Normal(NormalDocumentNameRef::new(s))
        }
    }
}

impl<'a> From<&'a str> for DocumentId {
    fn from(s: &'a str) -> Self {
        match DocumentIdRef::from(s) {
            DocumentIdRef::Normal(x) => DocumentId::Normal(x.into()),
            DocumentIdRef::Design(x) => DocumentId::Design(x.into()),
            DocumentIdRef::Local(x) => DocumentId::Local(x.into()),
        }
    }
}

impl From<String> for DocumentId {
    fn from(s: String) -> Self {
        DocumentId::from(s.as_str())
    }
}

impl<'a> From<&'a DocumentId> for DocumentIdRef<'a> {
    fn from(doc_id: &'a DocumentId) -> Self {
        doc_id.as_ref()
    }
}

impl<'a> From<DocumentIdRef<'a>> for DocumentId {
    fn from(doc_id: DocumentIdRef<'a>) -> Self {
        match doc_id {
            DocumentIdRef::Normal(x) => DocumentId::Normal(x.into()),
            DocumentIdRef::Design(x) => DocumentId::Design(x.into()),
            DocumentIdRef::Local(x) => DocumentId::Local(x.into()),
        }
    }
}

impl<'a> From<NormalDocumentNameRef<'a>> for DocumentIdRef<'a> {
    fn from(doc_name: NormalDocumentNameRef<'a>) -> Self {
        DocumentIdRef::Normal(doc_name)
    }
}

impl<'a> From<&'a NormalDocumentName> for DocumentIdRef<'a> {
    fn from(doc_name: &'a NormalDocumentName) -> Self {
        DocumentIdRef::Normal(doc_name.into())
    }
}

impl<'a> From<DesignDocumentNameRef<'a>> for DocumentIdRef<'a> {
    fn from(doc_name: DesignDocumentNameRef<'a>) -> Self {
        DocumentIdRef::Design(doc_name)
    }
}

impl<'a> From<&'a DesignDocumentName> for DocumentIdRef<'a> {
    fn from(doc_name: &'a DesignDocumentName) -> Self {
        DocumentIdRef::Design(doc_name.into())
    }
}

impl<'a> From<LocalDocumentNameRef<'a>> for DocumentIdRef<'a> {
    fn from(doc_name: LocalDocumentNameRef<'a>) -> Self {
        DocumentIdRef::Local(doc_name)
    }
}

impl<'a> From<&'a LocalDocumentName> for DocumentIdRef<'a> {
    fn from(doc_name: &'a LocalDocumentName) -> Self {
        DocumentIdRef::Local(doc_name.into())
    }
}

impl<'a> From<NormalDocumentNameRef<'a>> for DocumentId {
    fn from(doc_name: NormalDocumentNameRef<'a>) -> Self {
        DocumentId::Normal(doc_name.into())
    }
}

impl From<NormalDocumentName> for DocumentId {
    fn from(doc_name: NormalDocumentName) -> Self {
        DocumentId::Normal(doc_name)
    }
}

impl<'a> From<DesignDocumentNameRef<'a>> for DocumentId {
    fn from(doc_name: DesignDocumentNameRef<'a>) -> Self {
        DocumentId::Design(doc_name.into())
    }
}

impl From<DesignDocumentName> for DocumentId {
    fn from(doc_name: DesignDocumentName) -> Self {
        DocumentId::Design(doc_name)
    }
}

impl<'a> From<LocalDocumentNameRef<'a>> for DocumentId {
    fn from(doc_name: LocalDocumentNameRef<'a>) -> Self {
        DocumentId::Local(doc_name.into())
    }
}

impl From<LocalDocumentName> for DocumentId {
    fn from(doc_name: LocalDocumentName) -> Self {
        DocumentId::Local(doc_name)
    }
}

#[doc(hidden)]
impl<'a> serde::Serialize for DocumentIdRef<'a> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        self.to_string().serialize(serializer)
    }
}

#[doc(hidden)]
impl serde::Serialize for DocumentId {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        self.to_string().serialize(serializer)
    }
}

#[doc(hidden)]
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
                Ok(encoded.into())
            }

            fn visit_string<E>(&mut self, encoded: String) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                Ok(encoded.into())
            }
        }

        deserializer.deserialize(Visitor)
    }
}

#[cfg(test)]
mod tests {

    use prelude_impl::*;
    use serde_json;

    #[test]
    fn document_id_ref_from_str_ref_normal() {
        let expected = DocumentIdRef::Normal(NormalDocumentNameRef::new("foo"));
        assert_eq!(expected, DocumentIdRef::from("foo"));
    }

    #[test]
    fn document_id_ref_from_str_ref_design() {
        let expected = DocumentIdRef::Design(DesignDocumentNameRef::new("foo"));
        assert_eq!(expected, DocumentIdRef::from("_design/foo"));
    }

    #[test]
    fn document_id_ref_from_str_ref_local() {
        let expected = DocumentIdRef::Local(LocalDocumentNameRef::new("foo"));
        assert_eq!(expected, DocumentIdRef::from("_local/foo"));
    }

    #[test]
    fn document_id_ref_from_str_ref_invalid() {
        let expected = DocumentIdRef::Normal(NormalDocumentNameRef::new("_design"));
        assert_eq!(expected, DocumentIdRef::from("_design"));
    }

    #[test]
    fn document_id_ref_serialize() {
        let expected = serde_json::Value::String("_design/foo".into());
        let got = serde_json::to_value(&DocumentIdRef::Design("foo".into()));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_serialize() {
        let expected = serde_json::Value::String("_design/foo".into());
        let got = serde_json::to_value(&DocumentId::Design("foo".into()));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_id_deserialize() {
        let json = serde_json::Value::String("_design/foo".into());
        let expected = DocumentId::Design("foo".into());
        let got = serde_json::from_value(json).unwrap();
        assert_eq!(expected, got);
    }
}
