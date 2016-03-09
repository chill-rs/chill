use AttachmentName;
use base64;
use DocumentId;
use Error;
use mime;
use Revision;
use serde;
use serde_json;
use std;

#[derive(Debug, PartialEq)]
pub struct Document {
    doc_id: DocumentId,
    revision: Revision,
    deleted: bool,
    attachments: std::collections::HashMap<AttachmentName, Attachment>,
    content: serde_json::Value,
}

impl Document {
    pub fn id(&self) -> &DocumentId {
        &self.doc_id
    }

    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    pub fn get_content<C: serde::Deserialize>(&self) -> Result<C, Error> {
        serde_json::from_value(self.content.clone()).map_err(|e| Error::JsonDecode { cause: e })
    }

    pub fn set_content<C: serde::Serialize>(&mut self, new_content: &C) -> Result<(), Error> {
        self.content = serde_json::to_value(new_content);
        Ok(())
    }
}

impl serde::Serialize for Document {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        // FIXME: Eliminate all this cloning?
        //
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

impl serde::Deserialize for Document {
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
            type Value = Document;

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

                Ok(Document {
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
        deserializer.deserialize_struct("Document", FIELDS, Visitor)
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

impl serde::Serialize for Attachment {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        use serde::Serialize;
        match self {
            &Attachment::Saved(ref x) => x.serialize(serializer),
            &Attachment::Unsaved(ref x) => x.serialize(serializer),
        }
    }
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

impl serde::Serialize for SavedAttachment {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        struct Visitor;

        impl serde::ser::MapVisitor for Visitor {
            fn visit<S>(&mut self, serializer: &mut S) -> Result<Option<()>, S::Error>
                where S: serde::Serializer
            {
                try!(serializer.serialize_struct_elt("stub", true));
                Ok(None)
            }
        }

        serializer.serialize_struct("SavedAttachment", Visitor)
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
                    (Some(JsonDecodableBase64Blob(data)), None) => {
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

                let JsonDecodableContentType(content_type) = match content_type {
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

impl serde::Serialize for UnsavedAttachment {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        struct Visitor<'a>(&'a UnsavedAttachment);

        impl<'a> serde::ser::MapVisitor for Visitor<'a> {
            fn visit<S>(&mut self, serializer: &mut S) -> Result<Option<()>, S::Error>
                where S: serde::Serializer
            {
                // FIXME: Review: Mime implements Serialize, but it doesn't compile.

                let &mut Visitor(attachment) = self;
                let content_type = attachment.content_type.clone();
                try!(serializer.serialize_struct_elt("content_type", content_type.to_string()));
                try!(serializer.serialize_struct_elt("content",
                                                     &JsonEncodableBase64Blob(&attachment.content)));
                Ok(None)
            }
        }

        serializer.serialize_struct("UnsavedAttachment", Visitor(self))
    }
}

#[derive(Debug, PartialEq)]
struct JsonEncodableBase64Blob<'a>(&'a Vec<u8>);

impl<'a> serde::Serialize for JsonEncodableBase64Blob<'a> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        let &JsonEncodableBase64Blob(bytes) = self;
        String::from_utf8(base64::u8en(bytes).unwrap()).unwrap().serialize(serializer)
    }
}

#[derive(Debug, PartialEq)]
struct JsonDecodableBase64Blob(Vec<u8>);

impl serde::Deserialize for JsonDecodableBase64Blob {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = JsonDecodableBase64Blob;

            fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                use std::error::Error;
                let blob = try!(base64::u8de(value.as_bytes())
                                    .map_err(|e| E::invalid_value(e.description())));
                Ok(JsonDecodableBase64Blob(blob))
            }
        }

        deserializer.deserialize(Visitor)
    }
}

#[derive(Debug, PartialEq)]
pub struct JsonDecodableContentType(pub mime::Mime);

impl serde::Deserialize for JsonDecodableContentType {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = JsonDecodableContentType;

            fn visit_str<E>(&mut self, v: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                let m = try!(v.parse().map_err(|_| E::invalid_value("Bad MIME string")));
                Ok(JsonDecodableContentType(m))
            }
        }

        deserializer.deserialize(Visitor)
    }
}

#[cfg(test)]
#[derive(Debug)]
pub struct DocumentBuilder(Document);

#[cfg(test)]
impl DocumentBuilder {
    pub fn new<D: Into<DocumentId>>(doc_id: D, revision: Revision) -> Self {
        DocumentBuilder(Document {
            doc_id: doc_id.into(),
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

#[cfg(test)]
mod tests {

    use AttachmentName;
    use base64;
    use DocumentId;
    use Error;
    use Revision;
    use serde_json;
    use std;
    use super::{Attachment, AttachmentEncodingInfo, Document, JsonDecodableBase64Blob,
                JsonDecodableContentType, JsonEncodableBase64Blob, SavedAttachment,
                SavedAttachmentContent, UnsavedAttachment};

    #[test]
    fn document_get_content_ok() {

        let content = serde_json::builder::ObjectBuilder::new()
                          .insert("field_1", 42)
                          .insert("field_2", "foo")
                          .unwrap();

        let doc = Document {
            doc_id: DocumentId::from("document_id"),
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
            doc_id: DocumentId::from("document_id"),
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
            doc_id: DocumentId::from("document_id"),
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

    // FIXME: Implement this test.
    // #[test]
    // fn document_set_content_ok() {
    //     unimplemented!();
    // }

    // FIXME: Implement this test.
    // #[test]
    // fn document_set_content_ok_document_is_deleted() {
    //     unimplemented!();
    // }

    #[test]
    fn document_serialize_empty() {

        let document = Document {
            doc_id: DocumentId::from("document_id"),
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
            doc_id: DocumentId::from("document_id"),
            revision: Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap(),
            deleted: true, // This value should have no effect.
            attachments: {
                let mut m = std::collections::HashMap::new();
                m.insert(AttachmentName::from("attachment_1"), {
                    Attachment::Saved(SavedAttachment {
                        content_type: mime!(Text / Plain),
                        digest: "md5-XNdWXQ0FO9vPx7skS0GuYA==".to_string(),
                        sequence_number: 17,
                        content: SavedAttachmentContent::Bytes({
                            "Blah blah blah".to_string().into_bytes()
                        }),
                        encoding_info: None,
                    })
                });
                m.insert(AttachmentName::from("attachment_2"), {
                    Attachment::Unsaved(UnsavedAttachment {
                        content_type: mime!(Text / Html),
                        content: "<p>Yak yak yak</p>".to_string().into_bytes(),
                    })
                });
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
    fn document_deserialize_ok_as_minimum() {

        let expected = Document {
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
    fn document_deserialize_ok_as_deleted() {

        let expected = Document {
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
    fn document_deserialize_ok_with_content() {

        let expected = Document {
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
    fn document_deserialize_ok_with_attachments() {

        let expected = Document {
            doc_id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            deleted: false,
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
    fn document_deserialize_nok_missing_id() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_rev", "42-1234567890abcdef1234567890abcdef")
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<Document>(&source);
        expect_json_error_missing_field!(got, "_id");
    }

    #[test]
    fn document_deserialize_nok_missing_rev() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_id", "document_id")
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<Document>(&source);
        expect_json_error_missing_field!(got, "_rev");
    }

    #[test]
    fn attachment_serialize_saved() {

        let attachment = Attachment::Saved(SavedAttachment {
            content_type: mime!(Text / Plain),
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 17,
            content: SavedAttachmentContent::Bytes("This is the attachment."
                                                       .to_string()
                                                       .into_bytes()),
            encoding_info: None,
        });

        let encoded = serde_json::to_string(&attachment).unwrap();

        let expected = serde_json::builder::ObjectBuilder::new()
                           .insert("stub", true)
                           .unwrap();
        let got = serde_json::from_str(&encoded).unwrap();

        assert_eq!(expected, got);
    }

    #[test]
    fn attachment_serialize_unsaved() {

        let content = "This is the attachment.";

        let attachment = Attachment::Unsaved(UnsavedAttachment {
            content_type: mime!(Text / Plain),
            content: content.to_string().into_bytes(),
        });

        let encoded = serde_json::to_string(&attachment).unwrap();

        let expected = serde_json::builder::ObjectBuilder::new()
                           .insert("content_type", "text/plain")
                           .insert("content", base64::encode(content).unwrap())
                           .unwrap();

        let got = serde_json::from_str(&encoded).unwrap();

        assert_eq!(expected, got);
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
    fn saved_attachment_serialize() {

        let attachment = SavedAttachment {
            content_type: mime!(Text / Plain),
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 17,
            content: SavedAttachmentContent::Bytes("This is the attachment."
                                                       .to_string()
                                                       .into_bytes()),
            encoding_info: None,
        };

        let encoded = serde_json::to_string(&attachment).unwrap();

        let expected = serde_json::builder::ObjectBuilder::new()
                           .insert("stub", true)
                           .unwrap();
        let got = serde_json::from_str(&encoded).unwrap();

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
    fn unsaved_attachment_serialize() {

        let content = "This is the attachment.";

        let attachment = UnsavedAttachment {
            content_type: mime!(Text / Plain),
            content: content.to_string().into_bytes(),
        };

        let encoded = serde_json::to_string(&attachment).unwrap();

        let expected = serde_json::builder::ObjectBuilder::new()
                           .insert("content_type", "text/plain")
                           .insert("content", base64::encode(content).unwrap())
                           .unwrap();

        let got = serde_json::from_str(&encoded).unwrap();

        assert_eq!(expected, got);
    }

    #[test]
    fn json_base64_blob_serialize() {
        let bytes = "Blah blah blah".to_string().into_bytes();
        let encoded = serde_json::to_string(&JsonEncodableBase64Blob(&bytes)).unwrap();
        assert_eq!(r#""QmxhaCBibGFoIGJsYWg=""#, encoded);
    }

    #[test]
    fn json_base64_blob_deserialize_ok() {
        let expected = JsonDecodableBase64Blob("hello".to_owned().into_bytes());
        let source = serde_json::Value::String("aGVsbG8=".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn json_base64_blob_deserialize_nok_bad_base64() {
        let source = serde_json::Value::String("% percent signs are invalid in base64 %"
                                                   .to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<JsonDecodableBase64Blob>(&source);
        expect_json_error_invalid_value!(got);
    }

    #[test]
    fn json_content_type_deserialize_ok() {
        let expected = JsonDecodableContentType(mime!(Application / Json));
        let source = serde_json::Value::String("application/json".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn json_content_type_deserialize_nok_bad_mime() {
        let source = serde_json::Value::String("bad mime".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<JsonDecodableContentType>(&source);
        expect_json_error_invalid_value!(got);
    }
}
