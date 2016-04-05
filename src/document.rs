use prelude_impl::*;
use serde;
use serde_json;
use std;

/// Contains a specific version of a document.
///
/// A `Document` is an in-memory representation of a document, including its
/// content, attachments, and meta-information.
///
/// A `Document` may represent a document of any type: normal, design, or local.
#[derive(Debug, PartialEq)]
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
            doc_path: DocumentPath::new(db_name, doc.doc_id),
            revision: doc.revision,
            deleted: doc.deleted,
            attachments: doc.attachments,
            content: doc.content,
        }
    }

    #[doc(hidden)]
    pub fn database_name(&self) -> &DatabaseName {
        self.doc_path.database_name()
    }

    // FIXME: Should this be deprecated? Should we expose only paths, not ids?
    // What's the value in exposing an id instead of a path?
    /// Returns the document's id.
    pub fn id(&self) -> &DocumentId {
        self.doc_path.document_id()
    }

    #[doc(hidden)]
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
    pub fn get_content<C: serde::Deserialize>(&self) -> Result<C, Error> {
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
        self.content = serde_json::to_value(new_content);
        Ok(())
    }
}

#[doc(hidden)]
impl serde::Serialize for Document {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        // Serde requires structure field names to have static lifetimes.
        // However, our document content is dynamic. As a workaround, we construct a
        // serde_json::Value instance containing both the document's content and
        // its attachments.

        let mut value = self.content.clone();

        if let serde_json::Value::Object(ref mut fields) = value {
            if !self.attachments.is_empty() {
                let mut attachments = std::collections::BTreeMap::new();
                for (name, attachment) in self.attachments.iter() {
                    attachments.insert(String::from(name.clone()),
                                       serde_json::to_value(attachment));
                }
                fields.insert("_attachments".to_string(),
                              serde_json::Value::Object(attachments));
            }
        } else {
            panic!("Document content is not a JSON object");
        }

        value.serialize(serializer)
    }
}

// JsonDecodableDocument is necessary because the Document type is not
// decodable. It's not decodable because it requires a database name, which is
// not known at decode-time.
#[derive(Debug, PartialEq)]
pub struct JsonDecodableDocument {
    doc_id: DocumentId,
    revision: Revision,
    deleted: bool,
    attachments: std::collections::HashMap<AttachmentName, Attachment>,
    content: serde_json::Value,
}

impl serde::Deserialize for JsonDecodableDocument {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        enum Field {
            Attachments,
            Content(String),
            Deleted,
            Id,
            Rev,
        }

        impl serde::Deserialize for Field {
            fn deserialize<D>(deserializer: &mut D) -> Result<Field, D::Error>
                where D: serde::Deserializer
            {
                struct Visitor;

                impl serde::de::Visitor for Visitor {
                    type Value = Field;

                    fn visit_str<E>(&mut self, value: &str) -> Result<Field, E>
                        where E: serde::de::Error
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

                deserializer.deserialize(Visitor)
            }
        }

        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = JsonDecodableDocument;

            fn visit_map<V>(&mut self, mut visitor: V) -> Result<Self::Value, V::Error>
                where V: serde::de::MapVisitor
            {
                let mut attachments = None;
                let mut deleted = None;
                let mut id = None;
                let mut revision = None;
                let mut content_builder = serde_json::builder::ObjectBuilder::new();

                loop {
                    match try!(visitor.visit_key()) {
                        Some(Field::Attachments) => {
                            attachments = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Content(name)) => {
                            let value = Some(try!(visitor.visit_value::<serde_json::Value>()));
                            content_builder = content_builder.insert(name, value);
                        }
                        Some(Field::Deleted) => {
                            deleted = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Id) => {
                            id = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Rev) => {
                            revision = Some(try!(visitor.visit_value()));
                        }
                        None => {
                            break;
                        }
                    }
                }

                try!(visitor.end());

                Ok(JsonDecodableDocument {
                    doc_id: match id {
                        Some(x) => x,
                        None => try!(visitor.missing_field("_id")),
                    },
                    revision: match revision {
                        Some(x) => x,
                        None => try!(visitor.missing_field("_rev")),
                    },
                    deleted: deleted.unwrap_or(false),
                    attachments: attachments.unwrap_or(std::collections::HashMap::new()),
                    content: content_builder.unwrap(),
                })
            }
        }

        static FIELDS: &'static [&'static str] = &["_attachments", "_deleted", "_id", "_rev"];
        deserializer.deserialize_struct("JsonDecodableDocument", FIELDS, Visitor)
    }
}

#[cfg(test)]
#[derive(Debug)]
pub struct DocumentBuilder(Document);

#[cfg(test)]
impl DocumentBuilder {
    pub fn new<'a, P>(doc_path: P, revision: Revision) -> Self
        where P: IntoDocumentPath<'a>
    {
        DocumentBuilder(Document {
            doc_path: doc_path.into_document_path().unwrap().into(),
            revision: revision,
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: serde_json::builder::ObjectBuilder::new().unwrap(),
        })
    }

    pub fn unwrap(self) -> Document {
        let DocumentBuilder(doc) = self;
        doc
    }

    pub fn with_content<C: serde::Serialize>(mut self, new_content: &C) -> Self {
        {
            let DocumentBuilder(ref mut doc) = self;
            doc.content = serde_json::to_value(new_content);
        }
        self
    }

    pub fn build_content<F>(self, f: F) -> Self
        where F: FnOnce(serde_json::builder::ObjectBuilder) -> serde_json::builder::ObjectBuilder
    {
        self.with_content(&f(serde_json::builder::ObjectBuilder::new()).unwrap())
    }
}

#[derive(Debug, PartialEq)]
pub struct WriteDocumentResponse {
    pub ok: bool,
    pub doc_id: DocumentId,
    pub revision: Revision,
}

impl serde::Deserialize for WriteDocumentResponse {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        enum Field {
            Id,
            Ok,
            Rev,
        }

        impl serde::Deserialize for Field {
            fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
                where D: serde::Deserializer
            {
                struct Visitor;

                impl serde::de::Visitor for Visitor {
                    type Value = Field;

                    fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E>
                        where E: serde::de::Error
                    {
                        match value {
                            "id" => Ok(Field::Id),
                            "ok" => Ok(Field::Ok),
                            "rev" => Ok(Field::Rev),
                            _ => Err(E::unknown_field(value)),
                        }
                    }
                }

                deserializer.deserialize(Visitor)
            }
        }

        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = WriteDocumentResponse;

            fn visit_map<V>(&mut self, mut visitor: V) -> Result<Self::Value, V::Error>
                where V: serde::de::MapVisitor
            {
                let mut id = None;
                let mut ok = None;
                let mut rev = None;
                loop {
                    match try!(visitor.visit_key()) {
                        Some(Field::Id) => {
                            id = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Ok) => {
                            ok = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Rev) => {
                            rev = Some(try!(visitor.visit_value()));
                        }
                        None => {
                            break;
                        }
                    }
                }

                try!(visitor.end());

                Ok(WriteDocumentResponse {
                    doc_id: match id {
                        Some(x) => x,
                        None => try!(visitor.missing_field("id")),
                    },
                    ok: match ok {
                        Some(x) => x,
                        None => try!(visitor.missing_field("ok")),
                    },
                    revision: match rev {
                        Some(x) => x,
                        None => try!(visitor.missing_field("rev")),
                    },
                })
            }
        }

        static FIELDS: &'static [&'static str] = &["id", "ok", "rev"];
        deserializer.deserialize_struct("WriteDocumentResponse", FIELDS, Visitor)
    }
}

#[cfg(test)]
mod tests {

    use base64;
    use prelude_impl::*;
    use serde_json;
    use std;

    #[test]
    fn document_get_content_ok() {

        let content = serde_json::builder::ObjectBuilder::new()
                          .insert("field_1", 42)
                          .insert("field_2", "foo")
                          .unwrap();

        let doc = Document {
            doc_path: DocumentPath::parse("/database/document_id").unwrap(),
            revision: "1-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: content.clone(),
        };

        let expected = content;
        let got = doc.get_content().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_get_content_ok_document_is_deleted() {

        let content = serde_json::builder::ObjectBuilder::new().unwrap();

        let doc = Document {
            doc_path: DocumentPath::parse("/database/document_id").unwrap(),
            revision: "1-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: true,
            attachments: std::collections::HashMap::new(),
            content: content.clone(),
        };

        let expected = content;
        let got = doc.get_content().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_get_content_nok_decode_error() {

        let doc = Document {
            doc_path: DocumentPath::parse("/database/document_id").unwrap(),
            revision: "1-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: serde_json::builder::ObjectBuilder::new().unwrap(),
        };

        match doc.get_content::<i32>() {
            Err(Error::JsonDecode { .. }) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn document_serialize_empty() {

        let document = Document {
            doc_path: DocumentPath::parse("/database/document_id").unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: true, // This value should have no effect.
            attachments: std::collections::HashMap::new(),
            content: serde_json::builder::ObjectBuilder::new().unwrap(),
        };

        let encoded = serde_json::to_string(&document).unwrap();
        let expected = serde_json::builder::ObjectBuilder::new().unwrap();
        let got = serde_json::from_str(&encoded).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_serialize_with_content_and_attachments() {

        let document = Document {
            doc_path: DocumentPath::parse("/database/document_id").unwrap(),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: true, // This value should have no effect.
            attachments: {
                let mut m = std::collections::HashMap::new();
                m.insert(AttachmentName::from("attachment_1"),
                         AttachmentBuilder::new_saved_with_content(mime!(Text / Plain),
                                                                   "md5-XNdWXQ0FO9vPx7skS0GuYA==",
                                                                   17,
                                                                   "Blah blah blah")
                             .unwrap());
                m.insert(AttachmentName::from("attachment_2"),
                         AttachmentBuilder::new_unsaved(mime!(Text / Html),
                                                        "<p>Yak yak yak</p>"
                                                            .to_string()
                                                            .into_bytes())
                             .unwrap());
                m
            },
            content: serde_json::builder::ObjectBuilder::new()
                         .insert("field_1", 17)
                         .insert("field_2", "hello")
                         .unwrap(),
        };

        let encoded = serde_json::to_string(&document).unwrap();

        let expected = serde_json::builder::ObjectBuilder::new()
                           .insert("field_1", 17)
                           .insert("field_2", "hello")
                           .insert_object("_attachments", |x| {
                               x.insert_object("attachment_1", |x| x.insert("stub", true))
                                .insert_object("attachment_2", |x| {
                                    x.insert("content_type", "text/html")
                                     .insert("content",
                                             base64::encode("<p>Yak yak yak</p>").unwrap())
                                })
                           })
                           .unwrap();

        let got = serde_json::from_str(&encoded).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn json_decodable_document_deserialize_ok_as_minimum() {

        let expected = JsonDecodableDocument {
            doc_id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: false,
            attachments: std::collections::HashMap::new(),
            content: serde_json::builder::ObjectBuilder::new().unwrap(),
        };

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_id", "document_id")
                         .insert("_rev", "42-1234567890abcdef1234567890abcdef")
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
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
            content: serde_json::builder::ObjectBuilder::new().unwrap(),
        };

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_id", "document_id")
                         .insert("_rev", "42-1234567890abcdef1234567890abcdef")
                         .insert("_deleted", true)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
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
            content: serde_json::builder::ObjectBuilder::new()
                         .insert("field_1", 42)
                         .insert("field_2", 17)
                         .unwrap(),
        };

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_id", "document_id")
                         .insert("_rev", "42-1234567890abcdef1234567890abcdef")
                         .insert("field_1", 42)
                         .insert("field_2", 17)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
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
                map.insert(AttachmentName::from("attachment_1"),
                           AttachmentBuilder::new_saved(mime!(Application / WwwFormUrlEncoded),
                                                        "md5-XNdWXQ0FO9vPx7skS0GuYA=="
                                                            .to_string(),
                                                        23,
                                                        517)
                               .unwrap());
                map
            },
            content: serde_json::builder::ObjectBuilder::new().unwrap(),
        };

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_id", "document_id")
                         .insert("_rev", "42-1234567890abcdef1234567890abcdef")
                         .insert_object("_attachments", |x| {
                             x.insert_object("attachment_1", |x| {
                                 x.insert("content_type", "application/x-www-form-urlencoded")
                                  .insert("length", 517)
                                  .insert("stub", true)
                                  .insert("digest", "md5-XNdWXQ0FO9vPx7skS0GuYA==")
                                  .insert("revpos", 23)
                             })
                         })
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn json_decodable_document_deserialize_nok_missing_id() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_rev", "42-1234567890abcdef1234567890abcdef")
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<JsonDecodableDocument>(&source);
        expect_json_error_missing_field!(got, "_id");
    }

    #[test]
    fn json_decodable_document_deserialize_nok_missing_rev() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_id", "document_id")
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<JsonDecodableDocument>(&source);
        expect_json_error_missing_field!(got, "_rev");
    }

    #[test]
    fn write_document_response_deserialize_ok_with_all_fields() {
        let expected = WriteDocumentResponse {
            doc_id: "foo".into(),
            ok: true,
            revision: "1-12345678123456781234567812345678".parse().unwrap(),
        };
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("id", "foo")
                         .insert("ok", true)
                         .insert("rev", "1-12345678123456781234567812345678")
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn write_document_response_deserialize_nok_missing_id_field() {
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("ok", true)
                         .insert("rev", "1-12345678123456781234567812345678")
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<WriteDocumentResponse>(&source);
        expect_json_error_missing_field!(got, "id");
    }

    #[test]
    fn write_document_response_deserialize_nok_missing_ok_field() {
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("id", "foo")
                         .insert("rev", "1-12345678123456781234567812345678")
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<WriteDocumentResponse>(&source);
        expect_json_error_missing_field!(got, "ok");
    }

    #[test]
    fn write_document_response_deserialize_nok_missing_rev_field() {
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("id", "foo")
                         .insert("ok", true)
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<WriteDocumentResponse>(&source);
        expect_json_error_missing_field!(got, "rev");
    }
}
