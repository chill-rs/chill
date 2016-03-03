use AttachmentName;
use base64;
use DatabaseName;
use DocumentId;
use Error;
use mime;
use Revision;
use serde;
use serde_json;
use std;
use transport::{HyperTransport, Transport};

// Base contains meta-information for all documents, including deleted
// documents.
#[derive(Debug)]
pub struct DocumentBase<T: Transport> {
    pub transport: std::sync::Arc<T>,
    pub db_name: DatabaseName,
    pub doc_id: DocumentId,
    pub revision: Revision,
}

// Extra contains meta-information for non-deleted documents that doesn't
// exist for deleted documents.
#[derive(Debug, Default)]
pub struct DocumentExtra {
    pub attachments: std::collections::HashMap<AttachmentName, Attachment>,
}

pub type Document = BasicDocument<HyperTransport>;

#[derive(Debug)]
pub enum BasicDocument<T: Transport> {
    #[doc(hidden)]
    Deleted {
        base: DocumentBase<T>,
    },

    #[doc(hidden)]
    Exists {
        base: DocumentBase<T>,
        extra: DocumentExtra,
        content: serde_json::Value,
    },
}

impl<T: Transport> BasicDocument<T> {
    #[doc(hidden)]
    pub fn from_serializable_document(transport: &std::sync::Arc<T>,
                                      db_name: &DatabaseName,
                                      doc: SerializableDocument)
                                      -> Self {

        let base = DocumentBase {
            transport: transport.clone(),
            db_name: db_name.clone(),
            doc_id: doc.id,
            revision: doc.revision,
        };

        match doc.deleted {
            true => BasicDocument::Deleted { base: base },
            false => {
                BasicDocument::Exists {
                    base: base,
                    extra: DocumentExtra { attachments: doc.attachments },
                    content: doc.content,
                }
            }

        }
    }

    pub fn database_name(&self) -> &DatabaseName {
        match self {
            &BasicDocument::Deleted { ref base, .. } => &base.db_name,
            &BasicDocument::Exists { ref base, .. } => &base.db_name,
        }
    }

    pub fn id(&self) -> &DocumentId {
        match self {
            &BasicDocument::Deleted { ref base, .. } => &base.doc_id,
            &BasicDocument::Exists { ref base, .. } => &base.doc_id,
        }
    }


    pub fn revision(&self) -> &Revision {
        match self {
            &BasicDocument::Deleted { ref base, .. } => &base.revision,
            &BasicDocument::Exists { ref base, .. } => &base.revision,
        }
    }

    pub fn get_content<C: serde::Deserialize>(&self) -> Result<C, Error> {
        match self {
            &BasicDocument::Deleted { .. } => Err(Error::DocumentIsDeleted),
            &BasicDocument::Exists { ref content, .. } => {
                serde_json::from_value(content.clone()).map_err(|e| Error::JsonDecode { cause: e })
            }
        }
    }

    pub fn set_content<C: serde::Serialize>(&mut self, new_content: &C) -> Result<(), Error> {
        match self {
            &mut BasicDocument::Deleted { .. } => Err(Error::DocumentIsDeleted),
            &mut BasicDocument::Exists { ref mut content, .. } => {
                *content = serde_json::to_value(new_content);
                Ok(())
            }
        }
    }
}

#[derive(Debug, PartialEq)]
struct AttachmentEncodingInfo {
    encoded_length: u64,
    encoding: String,
}

#[derive(Debug, PartialEq)]
pub enum Attachment {
    Saved(SavedAttachment),
    Unsaved(UnsavedAttachment),
}

impl serde::Deserialize for Attachment {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        Ok(Attachment::Saved(try!(SavedAttachment::deserialize(deserializer))))
    }
}

#[derive(Debug, PartialEq)]
enum SavedAttachmentContent {
    LengthOnly(u64),
    Bytes(Vec<u8>),
}

#[derive(Debug, PartialEq)]
pub struct SavedAttachment {
    content_type: mime::Mime,
    digest: String,
    sequence_number: u64,
    content: SavedAttachmentContent,
    encoding_info: Option<AttachmentEncodingInfo>,
}

impl SavedAttachment {
    pub fn content_type(&self) -> &mime::Mime {
        &self.content_type
    }

    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    pub fn content_length(&self) -> u64 {
        match self.content {
            SavedAttachmentContent::LengthOnly(length) => length,
            SavedAttachmentContent::Bytes(ref bytes) => bytes.len() as u64,
        }
    }

    pub fn content_bytes(&self) -> Option<&[u8]> {
        match self.content {
            SavedAttachmentContent::LengthOnly(..) => None,
            SavedAttachmentContent::Bytes(ref bytes) => Some(&bytes),
        }
    }
}

impl serde::Deserialize for SavedAttachment {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        enum Field {
            ContentType,
            Data,
            Digest,
            EncodedLength,
            Encoding,
            Length,
            Revpos,
            Stub,
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
                            "content_type" => Ok(Field::ContentType),
                            "data" => Ok(Field::Data),
                            "digest" => Ok(Field::Digest),
                            "encoded_length" => Ok(Field::EncodedLength),
                            "encoding" => Ok(Field::Encoding),
                            "length" => Ok(Field::Length),
                            "revpos" => Ok(Field::Revpos),
                            "stub" => Ok(Field::Stub),
                            _ => Err(E::unknown_field(value)),
                        }
                    }
                }

                deserializer.deserialize(Visitor)
            }
        }

        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = SavedAttachment;

            fn visit_map<V>(&mut self, mut visitor: V) -> Result<Self::Value, V::Error>
                where V: serde::de::MapVisitor
            {
                let mut content_type = None;
                let mut data = None;
                let mut digest = None;
                let mut encoded_length = None;
                let mut encoding = None;
                let mut length = None;
                let mut revpos = None;

                loop {
                    match try!(visitor.visit_key()) {
                        Some(Field::ContentType) => {
                            content_type = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Data) => {
                            data = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Digest) => {
                            digest = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::EncodedLength) => {
                            encoded_length = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Encoding) => {
                            encoding = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Length) => {
                            length = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Revpos) => {
                            revpos = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Stub) => {
                            // Ignore this field.
                            try!(visitor.visit_value::<bool>());
                        }
                        None => {
                            break;
                        }
                    }
                }

                try!(visitor.end());

                let content = match (data, length) {
                    (Some(SerializableBase64Blob(data)), None) => {
                        SavedAttachmentContent::Bytes(data)
                    }
                    (None, Some(length)) => SavedAttachmentContent::LengthOnly(length),
                    (None, None) => {
                        use serde::de::Error;
                        return Err(V::Error::missing_field("length"));
                    }
                    (Some(_), Some(_)) => {
                        use serde::de::Error;
                        return Err(V::Error::unknown_field("data"));
                    }
                };

                let SerializableContentType(content_type) = match content_type {
                    Some(x) => x,
                    None => try!(visitor.missing_field("content_type")),
                };

                let digest = match digest {
                    Some(x) => x,
                    None => try!(visitor.missing_field("digest")),
                };

                let encoding_info = match (encoded_length, encoding) {
                    (None, None) => None,
                    (Some(encoded_length), Some(encoding)) => {
                        Some(AttachmentEncodingInfo {
                            encoded_length: encoded_length,
                            encoding: encoding,
                        })
                    }
                    (None, _) => {
                        use serde::de::Error;
                        return Err(V::Error::missing_field("encoded_info"));
                    }
                    (_, None) => {
                        use serde::de::Error;
                        return Err(V::Error::missing_field("encoding"));
                    }
                };

                let sequence_number = match revpos {
                    Some(x) => x,
                    None => try!(visitor.missing_field("revpos")),
                };

                Ok(SavedAttachment {
                    content: content,
                    content_type: content_type,
                    digest: digest,
                    encoding_info: encoding_info,
                    sequence_number: sequence_number,
                })
            }
        }

        static FIELDS: &'static [&'static str] = &["content_type",
                                                   "data",
                                                   "digest",
                                                   "encoded_length",
                                                   "encoding",
                                                   "length",
                                                   "revpos",
                                                   "stub"];
        deserializer.deserialize_struct("SavedAttachment", FIELDS, Visitor)
    }
}

#[derive(Debug, PartialEq)]
pub struct UnsavedAttachment {
    content_type: mime::Mime,
    content: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct SerializableDocument {
    pub id: DocumentId,
    pub revision: Revision,
    pub attachments: std::collections::HashMap<AttachmentName, Attachment>,
    pub deleted: bool,
    pub content: serde_json::Value,
}

impl serde::Deserialize for SerializableDocument {
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
                            "_deleted" => Ok(Field::Deleted),
                            "_id" => Ok(Field::Id),
                            "_rev" => Ok(Field::Rev),
                            "_attachments" => Ok(Field::Attachments),
                            _ => Ok(Field::Content(value.to_string())),
                        }
                    }
                }

                deserializer.deserialize(Visitor)
            }
        }

        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = SerializableDocument;

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

                let attachments = attachments.unwrap_or(std::collections::HashMap::new());
                let deleted = deleted.unwrap_or(false);

                let id = match id {
                    Some(x) => x,
                    None => try!(visitor.missing_field("_id")),
                };

                let revision = match revision {
                    Some(x) => x,
                    None => try!(visitor.missing_field("_rev")),
                };


                Ok(SerializableDocument {
                    attachments: attachments,
                    deleted: deleted,
                    id: id,
                    revision: revision,
                    content: content_builder.unwrap(),
                })
            }
        }

        static FIELDS: &'static [&'static str] = &["_attachments", "_deleted", "_id", "_rev"];
        deserializer.deserialize_struct("SerializableDocument", FIELDS, Visitor)
    }
}

#[derive(Debug, PartialEq)]
struct SerializableBase64Blob(Vec<u8>);

impl serde::Deserialize for SerializableBase64Blob {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = SerializableBase64Blob;

            fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                use std::error::Error;
                let blob = try!(base64::u8de(value.as_bytes())
                                    .map_err(|e| E::invalid_value(e.description())));
                Ok(SerializableBase64Blob(blob))
            }
        }

        deserializer.deserialize(Visitor)
    }
}

#[derive(Debug, PartialEq)]
pub struct SerializableContentType(pub mime::Mime);

impl serde::Deserialize for SerializableContentType {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = SerializableContentType;

            fn visit_str<E>(&mut self, v: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                let m = try!(v.parse().map_err(|_| E::invalid_value("Bad MIME string")));
                Ok(SerializableContentType(m))
            }
        }

        deserializer.deserialize(Visitor)
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

    use DatabaseName;
    use DocumentId;
    use Error;
    use Revision;
    use serde_json;
    use std;
    use super::{Attachment, AttachmentEncodingInfo, BasicDocument, DocumentBase, DocumentExtra,
                SavedAttachment, SavedAttachmentContent, SerializableBase64Blob,
                SerializableContentType, SerializableDocument, WriteDocumentResponse};
    use transport::MockTransport;

    fn new_mock_base<N, I>(db_name: N, doc_id: I, revision: Revision) -> DocumentBase<MockTransport>
        where N: Into<DatabaseName>,
              I: Into<DocumentId>
    {
        DocumentBase {
            transport: std::sync::Arc::new(MockTransport::new()),
            db_name: db_name.into(),
            doc_id: doc_id.into(),
            revision: revision,
        }
    }

    #[test]
    fn document_get_content_ok() {

        let content = serde_json::builder::ObjectBuilder::new()
                          .insert("field_1", 42)
                          .insert("field_2", "foo")
                          .unwrap();

        let doc = BasicDocument::Exists {
            base: new_mock_base("database_name",
                                "document_id",
                                "1-1234567890abcdef1234567890abcdef".parse().unwrap()),
            extra: DocumentExtra { attachments: std::collections::HashMap::new() },
            content: content.clone(),
        };

        let expected = content;
        let got = doc.get_content().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_get_content_nok_document_is_deleted() {

        let doc = BasicDocument::Deleted {
            base: new_mock_base("database_name",
                                "document_id",
                                "1-1234567890abcdef1234567890abcdef".parse().unwrap()),
        };

        match doc.get_content::<serde_json::Value>() {
            Err(Error::DocumentIsDeleted) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn document_get_content_nok_decode_error() {

        let doc = BasicDocument::Exists {
            base: new_mock_base("database_name",
                                "document_id",
                                "1-1234567890abcdef1234567890abcdef".parse().unwrap()),
            extra: Default::default(),
            content: serde_json::builder::ObjectBuilder::new().unwrap(),
        };

        match doc.get_content::<i32>() {
            Err(Error::JsonDecode { .. }) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn attachment_deserialize_ok() {

        let expected = Attachment::Saved(SavedAttachment {
            content_type: "text/plain".parse().unwrap(),
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 11,
            content: SavedAttachmentContent::LengthOnly(5),
            encoding_info: None,
        });

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("content_type", "text/plain")
                         .insert("digest", "md5-iMaiC8wqiFlD2NjLTemvCQ==")
                         .insert("length", 5)
                         .insert("revpos", 11)
                         .insert("stub", true)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn saved_attachment_deserialize_ok_as_stub() {

        let expected = SavedAttachment {
            content_type: "text/plain".parse().unwrap(),
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 11,
            content: SavedAttachmentContent::LengthOnly(5),
            encoding_info: None,
        };

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("content_type", "text/plain")
                         .insert("digest", "md5-iMaiC8wqiFlD2NjLTemvCQ==")
                         .insert("length", 5)
                         .insert("revpos", 11)
                         .insert("stub", true)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn saved_attachment_deserialize_ok_as_stub_with_encoding_info() {

        let expected = SavedAttachment {
            content_type: "text/plain".parse().unwrap(),
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 11,
            content: SavedAttachmentContent::LengthOnly(5),
            encoding_info: Some(AttachmentEncodingInfo {
                encoded_length: 25,
                encoding: "gzip".to_string(),
            }),
        };

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("content_type", "text/plain")
                         .insert("digest", "md5-iMaiC8wqiFlD2NjLTemvCQ==")
                         .insert("encoded_length", 25)
                         .insert("encoding", "gzip")
                         .insert("length", 5)
                         .insert("revpos", 11)
                         .insert("stub", true)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn saved_attachment_deserialize_ok_with_content_body() {

        let expected = SavedAttachment {
            content_type: "text/plain".parse().unwrap(),
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 11,
            content: SavedAttachmentContent::Bytes("hello".to_string().into_bytes()),
            encoding_info: None,
        };

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("content_type", "text/plain")
                         .insert("data", "aGVsbG8=")
                         .insert("digest", "md5-iMaiC8wqiFlD2NjLTemvCQ==")
                         .insert("revpos", 11)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        println!("JSON: {}", source);
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn saved_attachment_deserialize_nok_missing_content_type() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("digest", "md5-iMaiC8wqiFlD2NjLTemvCQ==")
                         .insert("length", 5)
                         .insert("revpos", 11)
                         .insert("stub", true)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SavedAttachment>(&source);
        expect_json_error_missing_field!(got, "content_type");
    }


    #[test]
    fn saved_attachment_deserialize_nok_missing_digest() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("content_type", "text/plain")
                         .insert("length", 5)
                         .insert("revpos", 11)
                         .insert("stub", true)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SavedAttachment>(&source);
        expect_json_error_missing_field!(got, "digest");
    }

    #[test]
    fn saved_attachment_deserialize_nok_missing_revpos() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("content_type", "text/plain")
                         .insert("digest", "md5-iMaiC8wqiFlD2NjLTemvCQ==")
                         .insert("length", 5)
                         .insert("stub", true)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SavedAttachment>(&source);
        expect_json_error_missing_field!(got, "revpos");
    }

    #[test]
    fn serializable_document_deserialize_ok_as_minimum() {

        let expected = SerializableDocument {
            id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            attachments: std::collections::HashMap::new(),
            deleted: false,
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
    fn serializable_document_deserialize_ok_as_deleted() {

        let expected = SerializableDocument {
            id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            attachments: std::collections::HashMap::new(),
            deleted: true,
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
    fn serializable_document_deserialize_ok_with_content() {

        let expected = SerializableDocument {
            id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            attachments: std::collections::HashMap::new(),
            deleted: false,
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
    fn serializable_document_deserialize_ok_with_attachments() {

        let expected = SerializableDocument {
            id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            attachments: {
                let mut map = std::collections::HashMap::new();
                map.insert(String::from("attachment_1"),
                           Attachment::Saved(SavedAttachment {
                               content_type: mime!(Application / WwwFormUrlEncoded),
                               digest: "md5-XNdWXQ0FO9vPx7skS0GuYA==".to_string(),
                               sequence_number: 23,
                               content: SavedAttachmentContent::LengthOnly(517),
                               encoding_info: None,
                           }));
                map
            },
            deleted: false,
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
    fn serializable_document_deserialize_nok_missing_id() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_rev", "42-1234567890abcdef1234567890abcdef")
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SerializableDocument>(&source);
        expect_json_error_missing_field!(got, "_id");
    }

    #[test]
    fn serializable_document_deserialize_nok_missing_rev() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_id", "document_id")
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SerializableDocument>(&source);
        expect_json_error_missing_field!(got, "_rev");
    }

    #[test]
    fn serializable_base64_blob_deserialize_ok() {
        let expected = SerializableBase64Blob("hello".to_owned().into_bytes());
        let source = serde_json::Value::String("aGVsbG8=".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn serializable_base64_blob_deserialize_nok_bad_base64() {
        let source = serde_json::Value::String("% percent signs are invalid in base64 %"
                                                   .to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SerializableBase64Blob>(&source);
        expect_json_error_invalid_value!(got);
    }

    #[test]
    fn serializable_content_type_deserialize_ok() {
        let expected = SerializableContentType(mime!(Application / Json));
        let source = serde_json::Value::String("application/json".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn serializable_content_type_deserialize_nok_bad_mime() {
        let source = serde_json::Value::String("bad mime".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SerializableContentType>(&source);
        expect_json_error_invalid_value!(got);
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
