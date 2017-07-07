use {Attachment, AttachmentName, AttachmentPath, DatabaseName, DocumentId, DocumentPath, Error, IntoDocumentPath,
     Revision, mime, serde, serde_json, std};
use attachment::AttachmentBuilder;

/// Contains a specific version of a document.
///
/// A `Document` is an in-memory representation of a document, including its
/// content, attachments, and meta-information.
///
/// A `Document` may represent a document of any type: normal, design, or local.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    doc_path: DocumentPath,
    revision: Revision,
    deleted: bool,
    attachments: std::collections::HashMap<AttachmentName, Attachment>,
    content: serde_json::Value,
}

impl Document {
    #[doc(hidden)]
    pub fn new_from_decoded(db_name: DatabaseName, doc: JsonDecodableDocument) -> Self {
        Document {
            doc_path: DocumentPath::from((db_name, doc.doc_id)),
            revision: doc.revision,
            deleted: doc.deleted,
            attachments: doc.attachments,
            content: doc.content,
        }
    }

    pub fn path(&self) -> &DocumentPath {
        &self.doc_path
    }

    /// Returns the document's revision.
    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    /// Returns `true` if and only if the document is deleted.
    ///
    /// Normally, the CouchDB server returns a `NotFound` error if the
    /// application attempts to read a deleted document. However, the
    /// application may specify a revision when reading the document, and, if
    /// the revision marks when the document was deleted, then the server will
    /// respond successfully with a `Document` “stub” marked as deleted.
    ///
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    /// Decodes and returns the document content, from a JSON object into a Rust
    /// type.
    pub fn get_content<C>(&self) -> Result<C, Error>
    where
        for<'de> C: serde::Deserialize<'de>,
    {
        serde_json::from_value(self.content.clone()).map_err(|e| Error::JsonDecode { cause: e })
    }

    /// Encodes the document content, from a Rust type into a JSON object.
    ///
    /// The `set_content` method modifies the `Document` instance but doesn't
    /// update the document on the CouchDB server. To update the document on the
    /// server, the application must use the `UpdateDocument` action upon the
    /// modified `Document`.
    ///
    pub fn set_content<C: serde::Serialize>(&mut self, new_content: &C) -> Result<(), Error> {
        self.content = serde_json::to_value(new_content).map_err(|e| {
            Error::JsonEncode { cause: e }
        })?;
        Ok(())
    }

    /// Returns the document's attachment of a given name, if the attachment
    /// exists.
    pub fn get_attachment<'a, A>(&self, att_name: A) -> Option<&Attachment>
    where
        A: Into<AttachmentName>,
    {
        let att_name = att_name.into();
        self.attachments.get(&att_name)
    }

    /// Creates or replaces the document's attachment of a given name.
    ///
    /// This method does _not_ change state on the CouchDB server, and the newly
    /// created attachment exists only in memory on the client-side. To store
    /// the document on the CouchDB server, the client must execute an action to
    /// update the document, at which time the client will send the attachment
    /// to the server.
    ///
    pub fn insert_attachment<'a, A>(&mut self, att_name: A, content_type: mime::Mime, content: Vec<u8>)
    where
        A: Into<AttachmentName>,
    {
        let att_name = att_name.into();
        self.attachments.insert(
            att_name,
            AttachmentBuilder::new_unsaved(content_type, content).unwrap(),
        );
    }

    /// Deletes the document's attachment of a given name, if the attachment
    /// exists.
    ///
    /// This method does _not_ change state on the CouchDB server, and the newly
    /// deleted attachment will continue to exist on the CouchDB server until
    /// the client executes an action to update the document.
    ///
    pub fn remove_attachment<'a, A>(&mut self, att_name: A)
    where
        A: Into<AttachmentName>,
    {
        let att_name = att_name.into();
        self.attachments.remove(&att_name);
    }

    /// Returns an iterator to all attachments to the document.
    pub fn attachments(&self) -> AttachmentIter {
        AttachmentIter {
            doc_path: &self.doc_path,
            inner: self.attachments.iter(),
        }
    }
}

#[doc(hidden)]
impl serde::Serialize for Document {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serde requires structure field names to have static lifetimes.
        // However, our document content is dynamic. As a workaround, we construct a
        // serde_json::Value instance containing both the document's content and
        // its attachments.

        let mut value = self.content.clone();

        if let serde_json::Value::Object(ref mut fields) = value {
            if !self.attachments.is_empty() {
                let mut attachments = serde_json::map::Map::new();
                for (name, attachment) in self.attachments.iter() {
                    attachments.insert(
                        String::from(name.clone()),
                        serde_json::to_value(attachment).unwrap(),
                    );
                }
                fields.insert(
                    "_attachments".to_string(),
                    serde_json::Value::Object(attachments),
                );
            }
        } else {
            panic!("Document content is not a JSON object");
        }

        value.serialize(serializer)
    }
}

pub struct AttachmentIter<'a> {
    doc_path: &'a DocumentPath,
    inner: std::collections::hash_map::Iter<'a, AttachmentName, Attachment>,
}

impl<'a> Iterator for AttachmentIter<'a> {
    type Item = (AttachmentPath, &'a Attachment);

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            None => None,
            Some((att_name, att)) => Some((
                AttachmentPath::from(
                    (self.doc_path.clone(), att_name.clone()),
                ),
                att,
            )),
        }
    }
}

#[cfg(test)]
mod document_tests {

    use super::*;
    use {Error, IntoDocumentPath, base64};

    #[test]
    fn get_content_ok() {

        let content = json!({
            "field_1": 42,
            "field_2": "foo",
        });

        let doc = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: "1-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: content.clone(),
        };

        let expected = content;
        let got: serde_json::Value = doc.get_content().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn get_content_ok_document_is_deleted() {

        let doc = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: "1-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: true,
            attachments: std::collections::HashMap::new(),
            content: json!({}),
        };

        let expected = json!({});
        let got: serde_json::Value = doc.get_content().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn get_content_nok_decode_error() {

        let doc = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: "1-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: json!({}),
        };

        match doc.get_content::<i32>() {
            Err(Error::JsonDecode { .. }) => (),
            x => panic!("Got unexpected result {:?}", x),
        }
    }

    #[test]
    fn serialize_empty() {

        let document = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: true, // This value should have no effect.
            attachments: std::collections::HashMap::new(),
            content: json!({}),
        };

        let encoded = serde_json::to_string(&document).unwrap();
        let expected = json!({});
        let got: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn serialize_with_content_and_attachments() {

        let document = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: true, // This value should have no effect.
            attachments: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    AttachmentName::from("attachment_1"),
                    AttachmentBuilder::new_saved_with_content(
                        mime::TEXT_PLAIN,
                        "md5-XNdWXQ0FO9vPx7skS0GuYA==",
                        17,
                        "Blah blah blah",
                    ).unwrap(),
                );
                m.insert(
                    AttachmentName::from("attachment_2"),
                    AttachmentBuilder::new_unsaved(
                        mime::TEXT_HTML,
                        "<p>Yak yak yak</p>".to_string().into_bytes(),
                    ).unwrap(),
                );
                m
            },
            content: json!({
                "field_1": 17,
                "field_2": "hello",
            }),
        };

        let encoded = serde_json::to_string(&document).unwrap();

        let expected = json!({
            "field_1": 17,
            "field_2": "hello",
            "_attachments": {
                "attachment_1": {
                    "stub": true,
                },
                "attachment_2": {
                    "content_type": "text/html",
                    "data": base64::encode(b"<p>Yak yak yak</p>"),
                },
            },
        });

        let got: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn get_attachment_exists() {

        let attachment_1 = AttachmentBuilder::new_saved_with_content(
            mime::TEXT_PLAIN,
            "md5-XNdWXQ0FO9vPx7skS0GuYA=\
                                                                      =",
            17,
            "Blah blah blah",
        ).unwrap();

        let document = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: {
                let mut m = std::collections::HashMap::new();
                m.insert(AttachmentName::from("attachment_1"), attachment_1.clone());
                m
            },
            content: json!({}),
        };

        let got = document.get_attachment("attachment_1");
        assert_eq!(Some(&attachment_1), got);
    }

    #[test]
    fn get_attachment_no_exist() {

        let document = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: json!({}),
        };

        let got = document.get_attachment("attachment_1");
        assert_eq!(None, got);
    }

    #[test]
    fn insert_attachment_new() {

        let mut document = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: json!({}),
        };

        document.insert_attachment("foo", mime::TEXT_PLAIN, "This is the content.".into());
        document.insert_attachment(
            "bar",
            mime::TEXT_PLAIN,
            "This is the second attachment.".into(),
        );

        let expected = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    AttachmentName::from("foo"),
                    AttachmentBuilder::new_unsaved(mime::TEXT_PLAIN, "This is the content.").unwrap(),
                );
                m.insert(
                    AttachmentName::from("bar"),
                    AttachmentBuilder::new_unsaved(mime::TEXT_PLAIN, "This is the second attachment.").unwrap(),
                );
                m
            },
            content: json!({}),
        };

        assert_eq!(expected, document);
    }

    #[test]
    fn insert_attachment_replace() {

        let mut document = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: json!({}),
        };

        document.insert_attachment("foo", mime::TEXT_PLAIN, "This is the content.".into());
        document.insert_attachment(
            "foo",
            mime::TEXT_PLAIN,
            "This is the second attachment.".into(),
        );

        let expected = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    AttachmentName::from("foo"),
                    AttachmentBuilder::new_unsaved(mime::TEXT_PLAIN, "This is the second attachment.").unwrap(),
                );
                m
            },
            content: json!({}),
        };

        assert_eq!(expected, document);
    }

    #[test]
    fn remove_attachment_exists() {

        let mut document = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    AttachmentName::from("foo"),
                    AttachmentBuilder::new_unsaved(mime::TEXT_PLAIN, "This is the content.").unwrap(),
                );
                m
            },
            content: json!({}),
        };

        document.remove_attachment("foo");

        let expected = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: json!({}),
        };

        assert_eq!(expected, document);
    }

    #[test]
    fn remove_attachment_no_exist() {

        let mut document = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    AttachmentName::from("foo"),
                    AttachmentBuilder::new_unsaved(mime::TEXT_PLAIN, "This is the content.").unwrap(),
                );
                m
            },
            content: json!({}),
        };

        document.remove_attachment("bar");

        let expected = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    AttachmentName::from("foo"),
                    AttachmentBuilder::new_unsaved(mime::TEXT_PLAIN, "This is the content.").unwrap(),
                );
                m
            },
            content: json!({}),
        };

        assert_eq!(expected, document);
    }

    #[test]
    fn iterate_through_attachments() {

        let attachments = {
            let mut m = std::collections::HashMap::new();
            m.insert(
                AttachmentName::from("foo"),
                AttachmentBuilder::new_unsaved(mime::TEXT_PLAIN, "This is the content.").unwrap(),
            );
            m.insert(
                AttachmentName::from("bar"),
                AttachmentBuilder::new_unsaved(mime::TEXT_PLAIN, "This is the second attachment.").unwrap(),
            );
            m
        };

        let document = Document {
            doc_path: "/database/document_id".into_document_path().unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: false,
            attachments: attachments.clone(),
            content: json!({}),
        };

        let got = document
            .attachments()
            .map(|(path, attachment)| {
                (path.attachment_name().clone(), attachment.clone())
            })
            .collect();
        assert_eq!(attachments, got);
    }
}

// JsonDecodableDocument is necessary because the Document type is not
// decodable. It's not decodable because it requires a database name, which is
// not known at decode-time.
#[derive(Debug, PartialEq)]
pub struct JsonDecodableDocument {
    pub doc_id: DocumentId,
    pub revision: Revision,
    pub deleted: bool,
    pub attachments: std::collections::HashMap<AttachmentName, Attachment>,
    pub content: serde_json::Value,
}

impl<'de> serde::Deserialize<'de> for JsonDecodableDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        enum Field {
            Attachments,
            Content(String),
            Deleted,
            Id,
            Rev,
        }

        impl<'de> serde::Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct Visitor;

                impl<'de> serde::de::Visitor<'de> for Visitor {
                    type Value = Field;

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                        write!(f, "a CouchDB document field")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "_attachments" => Ok(Field::Attachments),
                            "_deleted" => Ok(Field::Deleted),
                            "_id" => Ok(Field::Id),
                            "_rev" => Ok(Field::Rev),
                            _ => Ok(Field::Content(value.to_string())),
                        }
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = JsonDecodableDocument;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                write!(f, "a CouchDb document object")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut attachments = None;
                let mut deleted = None;
                let mut id = None;
                let mut revision = None;
                let mut content_builder = serde_json::map::Map::new();

                while let Some(key) = visitor.next_key()? {
                    match key {
                        Field::Attachments => {
                            if attachments.is_some() {
                                return Err(serde::de::Error::duplicate_field("_attachments"));
                            }
                            attachments = Some(visitor.next_value()?);
                        }
                        Field::Content(name) => {
                            let value = visitor.next_value::<serde_json::Value>()?;
                            content_builder.insert(name, value);
                        }
                        Field::Deleted => {
                            if deleted.is_some() {
                                return Err(serde::de::Error::duplicate_field("_deleted"));
                            }
                            deleted = Some(visitor.next_value()?);
                        }
                        Field::Id => {
                            if id.is_some() {
                                return Err(serde::de::Error::duplicate_field("_id"));
                            }
                            id = Some(visitor.next_value()?);
                        }
                        Field::Rev => {
                            if revision.is_some() {
                                return Err(serde::de::Error::duplicate_field("_rev"));
                            }
                            revision = Some(visitor.next_value()?);
                        }
                    }
                }

                Ok(JsonDecodableDocument {
                    doc_id: match id {
                        Some(x) => x,
                        None => return Err(serde::de::Error::missing_field("_id")),
                    },
                    revision: match revision {
                        Some(x) => x,
                        None => return Err(serde::de::Error::missing_field("_rev")),
                    },
                    deleted: deleted.unwrap_or(false),
                    attachments: attachments.unwrap_or(std::collections::HashMap::new()),
                    content: serde_json::Value::Object(content_builder),
                })
            }
        }

        static FIELDS: &'static [&'static str] = &["_attachments", "_deleted", "_id", "_rev"];
        deserializer.deserialize_struct("JsonDecodableDocument", FIELDS, Visitor)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct WriteDocumentResponse {
    pub ok: bool,
    pub id: DocumentId,
    pub rev: Revision,
}

#[derive(Debug)]
pub struct DocumentBuilder(Document);

impl DocumentBuilder {
    pub fn new<P: IntoDocumentPath>(doc_path: P, revision: Revision) -> Self {
        DocumentBuilder(Document {
            doc_path: doc_path.into_document_path().unwrap().into(),
            revision: revision,
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: json!({}),
        })
    }

    pub fn unwrap(self) -> Document {
        let DocumentBuilder(doc) = self;
        doc
    }

    pub fn with_content<C: serde::Serialize>(mut self, new_content: &C) -> Self {
        {
            let DocumentBuilder(ref mut doc) = self;
            doc.content = serde_json::to_value(new_content).unwrap();
        }
        self
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use serde_json;

    #[test]
    fn json_decodable_document_deserialize_ok_as_minimum() {

        let expected = JsonDecodableDocument {
            doc_id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: json!({}),
        };

        let source = r#"{"_id":"document_id",
                         "_rev":"42-1234567890abcdef1234567890abcdef"}"#;

        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn json_decodable_document_deserialize_ok_as_deleted() {

        let expected = JsonDecodableDocument {
            doc_id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: true,
            attachments: std::collections::HashMap::new(),
            content: json!({}),
        };

        let source = r#"{"_id":"document_id",
                         "_rev":"42-1234567890abcdef1234567890abcdef",
                         "_deleted":true}"#;

        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn json_decodable_document_deserialize_ok_with_content() {

        let expected = JsonDecodableDocument {
            doc_id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: json!({
                "field_1": 42,
                "field_2": 17,
            }),
        };

        let source = r#"{"_id":"document_id",
                         "_rev":"42-1234567890abcdef1234567890abcdef",
                         "field_1": 42,
                         "field_2": 17}"#;

        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn json_decodable_document_deserialize_ok_with_attachments() {

        let expected = JsonDecodableDocument {
            doc_id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: false,
            attachments: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    AttachmentName::from("attachment_1"),
                    AttachmentBuilder::new_saved(
                        mime::APPLICATION_WWW_FORM_URLENCODED,
                        "md5-XNdWXQ0FO9vPx7skS0GuYA==".to_string(),
                        23,
                        517,
                    ).unwrap(),
                );
                map
            },
            content: json!({}),
        };

        let source = r#"{"_id":"document_id",
                         "_rev":"42-1234567890abcdef1234567890abcdef",
                         "_attachments": {
                             "attachment_1": {
                                 "content_type":"application/x-www-form-urlencoded",
                                 "length":517,
                                 "stub":true,
                                 "digest":"md5-XNdWXQ0FO9vPx7skS0GuYA==",
                                 "revpos":23
                             }
                         }}"#;

        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn json_decodable_document_deserialize_nok_missing_id() {
        let source = r#"{"_rev":"42-1234567890abcdef1234567890abcdef"}"#;
        match serde_json::from_str::<JsonDecodableDocument>(&source) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }

    #[test]
    fn json_decodable_document_deserialize_nok_missing_rev() {
        let source = r#"{"_id":"document_id"}"#;
        match serde_json::from_str::<JsonDecodableDocument>(&source) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }

    #[test]
    fn write_document_response_deserialize_ok_with_all_fields() {
        let expected = WriteDocumentResponse {
            ok: true,
            id: "foo".into(),
            rev: "1-12345678123456781234567812345678".parse().unwrap(),
        };
        let source = r#"{"id":"foo",
                         "ok":true,
                         "rev":"1-12345678123456781234567812345678"}"#;
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn write_document_response_deserialize_nok_missing_id_field() {
        let source = r#"{"ok":true,"rev":"1-12345678123456781234567812345678"}"#;
        match serde_json::from_str::<WriteDocumentResponse>(&source) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }

    #[test]
    fn write_document_response_deserialize_nok_missing_ok_field() {
        let source = r#"{"id":"foo","rev":"1-12345678123456781234567812345678"}"#;
        match serde_json::from_str::<WriteDocumentResponse>(&source) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }

    #[test]
    fn write_document_response_deserialize_nok_missing_rev_field() {
        let source = r#"{"id":"foo","ok":true}"#;
        match serde_json::from_str::<WriteDocumentResponse>(&source) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }
}
