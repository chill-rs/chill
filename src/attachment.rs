// FIXME: These types as too smart for Serde. Actual serialization and
// deserialization should use derived traits, and any higher-level types we use
// for the Chill's API should convert to and from those type.

use {base64, mime, serde, std};

#[derive(Clone, Debug, PartialEq)]
struct AttachmentEncodingInfo {
    encoded_length: u64,
    encoding: String,
}

/// Contains the meta-information and, optionally, content of an attachment.
///
#[derive(Clone, Debug, PartialEq)]
pub enum Attachment {
    #[doc(hidden)]
    Saved(SavedAttachment),

    #[doc(hidden)]
    Unsaved(UnsavedAttachment),
}

impl Attachment {
    /// Returns the attachment's content type.
    pub fn content_type(&self) -> &mime::Mime {
        match self {
            &Attachment::Saved(ref inner) => &inner.content_type,
            &Attachment::Unsaved(ref inner) => &inner.content_type,
        }
    }

    /// Returns the attachment's content size, in bytes.
    pub fn content_length(&self) -> u64 {
        match self {
            &Attachment::Saved(ref inner) => {
                match inner.content {
                    SavedAttachmentContent::LengthOnly(len) => len,
                    SavedAttachmentContent::Bytes(ref bytes) => bytes.len() as u64,
                }
            }
            &Attachment::Unsaved(ref inner) => inner.content.len() as u64,
        }
    }

    /// Returns the attachment's content, if available.
    ///
    /// An attachment's content is available if and only if the attachment is
    /// _not_ a stub. By default, the CouchDB server sends attachment stubs as
    /// part of a document when the client reads the document. The client may
    /// explicitly request the attachment content when reading the document to
    /// receive a full attachment, in which case this method will return `Some`
    /// instead of `None`. Also, if the client inserted the attachment via the
    /// `Document::insert_attachment` method, then the attachment contains
    /// content and this method will return `Some`.
    ///
    pub fn content(&self) -> Option<&Vec<u8>> {
        match self {
            &Attachment::Saved(ref inner) => {
                match inner.content {
                    SavedAttachmentContent::LengthOnly(..) => None,
                    SavedAttachmentContent::Bytes(ref bytes) => Some(bytes),
                }
            }
            &Attachment::Unsaved(ref inner) => Some(&inner.content),
        }
    }
}

#[doc(hidden)]
impl serde::Serialize for Attachment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            &Attachment::Saved(ref x) => x.serialize(serializer),
            &Attachment::Unsaved(ref x) => x.serialize(serializer),
        }
    }
}

#[doc(hidden)]
impl<'de> serde::Deserialize<'de> for Attachment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Attachment::Saved(
            try!(SavedAttachment::deserialize(deserializer)),
        ))
    }
}

#[derive(Clone, Debug, PartialEq)]
enum SavedAttachmentContent {
    LengthOnly(u64),
    Bytes(Vec<u8>),
}

#[doc(hidden)]
#[derive(Clone, Debug, PartialEq)]
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

// FIXME: Derive this?
impl serde::Serialize for SavedAttachment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SavedAttachment", 1)?;
        state.serialize_field("stub", &true)?;
        state.end()
    }
}

// FIXME: Derive this?
impl<'de> serde::Deserialize<'de> for SavedAttachment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
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

        impl<'de> serde::Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct Visitor;

                impl<'de> serde::de::Visitor<'de> for Visitor {
                    type Value = Field;

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                        write!(f, "a CouchDB attachment object field name")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
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
                            _ => Err(E::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = SavedAttachment;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                write!(f, "a CouchDB attachment object")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut content_type = None;
                let mut data = None;
                let mut digest = None;
                let mut encoded_length = None;
                let mut encoding = None;
                let mut length = None;
                let mut revpos = None;

                while let Some(key) = visitor.next_key()? {
                    match key {
                        Field::ContentType => {
                            if content_type.is_some() {
                                return Err(serde::de::Error::duplicate_field("content-type"));
                            }
                            content_type = Some(visitor.next_value()?);
                        }
                        Field::Data => {
                            if data.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data = Some(visitor.next_value()?);
                        }
                        Field::Digest => {
                            if digest.is_some() {
                                return Err(serde::de::Error::duplicate_field("digest"));
                            }
                            digest = Some(visitor.next_value()?);
                        }
                        Field::EncodedLength => {
                            if encoded_length.is_some() {
                                return Err(serde::de::Error::duplicate_field("encoded_length"));
                            }
                            encoded_length = Some(visitor.next_value()?);
                        }
                        Field::Encoding => {
                            if encoding.is_some() {
                                return Err(serde::de::Error::duplicate_field("encoding"));
                            }
                            encoding = Some(visitor.next_value()?);
                        }
                        Field::Length => {
                            if length.is_some() {
                                return Err(serde::de::Error::duplicate_field("length"));
                            }
                            length = Some(visitor.next_value()?);
                        }
                        Field::Revpos => {
                            if revpos.is_some() {
                                return Err(serde::de::Error::duplicate_field("revpos"));
                            }
                            revpos = Some(visitor.next_value()?);
                        }
                        Field::Stub => {
                            // Ignore this field.
                            visitor.next_value::<bool>()?;
                        }
                    }
                }

                let content = match (data, length) {
                    (Some(Base64JsonDecodable(data)), None) => SavedAttachmentContent::Bytes(data),
                    (None, Some(length)) => SavedAttachmentContent::LengthOnly(length),
                    (None, None) => {
                        use serde::de::Error;
                        return Err(V::Error::missing_field("length"));
                    }
                    (Some(_), Some(_)) => {
                        use serde::de::Error;
                        return Err(V::Error::unknown_field("data", FIELDS));
                    }
                };

                let ContentTypeJsonDecodable(content_type) = match content_type {
                    Some(x) => x,
                    None => return Err(serde::de::Error::missing_field("content_type")),
                };

                let digest = match digest {
                    Some(x) => x,
                    None => return Err(serde::de::Error::missing_field("digest")),
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
                    None => return Err(serde::de::Error::missing_field("revpos")),
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

        static FIELDS: &'static [&'static str] = &[
            "content_type",
            "data",
            "digest",
            "encoded_length",
            "encoding",
            "length",
            "revpos",
            "stub",
        ];
        deserializer.deserialize_struct("SavedAttachment", FIELDS, Visitor)
    }
}

#[doc(hidden)]
#[derive(Clone, Debug, PartialEq)]
pub struct UnsavedAttachment {
    content_type: mime::Mime,
    content: Vec<u8>,
}

impl serde::Serialize for UnsavedAttachment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let content_type = self.content_type.clone();
        let mut state = serializer.serialize_struct("UnsavedAttachment", 2)?;
        state.serialize_field(
            "content_type",
            &content_type.to_string(),
        )?;
        state.serialize_field(
            "data",
            &Base64JsonEncodable(&self.content),
        )?;
        state.end()
    }
}

#[derive(Debug, PartialEq)]
struct Base64JsonDecodable(Vec<u8>);

impl<'de> serde::Deserialize<'de> for Base64JsonDecodable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Base64JsonDecodable;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                write!(
                    f,
                    "a base64-encoded string containing CouchDB attachment data"
                )
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let blob = try!(base64::decode(value).map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Str(value), &self)
                }));
                Ok(Base64JsonDecodable(blob))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[derive(Debug, PartialEq)]
struct Base64JsonEncodable<'a>(&'a Vec<u8>);

impl<'a> serde::Serialize for Base64JsonEncodable<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let &Base64JsonEncodable(bytes) = self;
        base64::encode(bytes).serialize(serializer)
    }
}

#[derive(Debug, PartialEq)]
struct ContentTypeJsonDecodable(pub mime::Mime);

impl<'de> serde::Deserialize<'de> for ContentTypeJsonDecodable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = ContentTypeJsonDecodable;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                write!(f, "a string specifying a CouchDB attachment content-type")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let m = try!(v.parse().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Str(v), &self)
                }));
                Ok(ContentTypeJsonDecodable(m))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

/// Builds an attachment.
///
/// An `AttachmentBuilder` constructs an attachment as though the attachment
/// originated from the CouchDB server (as a saved attachment) or from the
/// client (as an unsaved attachment).
///
#[derive(Debug)]
pub struct AttachmentBuilder<M> {
    target: Attachment,
    _phantom: std::marker::PhantomData<M>,
}

/// Marks that an attachment is saved.
#[allow(dead_code)]
pub struct AttachmentIsSaved;

/// Marks that an attachment is unsaved.
#[allow(dead_code)]
pub struct AttachmentIsUnsaved;

#[allow(dead_code)]
impl AttachmentBuilder<AttachmentIsSaved> {
    /// Constructs a saved attachment as a stub.
    ///
    /// An attachment stub specifies the length of its content but does not
    /// store the content itself.
    ///
    pub fn new_saved<D>(content_type: mime::Mime, digest: D, sequence_number: u64, content_length: u64) -> Self
    where
        D: Into<String>,
    {
        AttachmentBuilder {
            target: Attachment::Saved(SavedAttachment {
                content_type: content_type,
                digest: digest.into(),
                sequence_number: sequence_number,
                content: SavedAttachmentContent::LengthOnly(content_length),
                encoding_info: None,
            }),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Constructs a saved attachment in full.
    pub fn new_saved_with_content<C, D>(content_type: mime::Mime, digest: D, sequence_number: u64, content: C) -> Self
    where
        C: Into<Vec<u8>>,
        D: Into<String>,
    {
        AttachmentBuilder {
            target: Attachment::Saved(SavedAttachment {
                content_type: content_type,
                digest: digest.into(),
                sequence_number: sequence_number,
                content: SavedAttachmentContent::Bytes(content.into()),
                encoding_info: None,
            }),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl AttachmentBuilder<AttachmentIsUnsaved> {
    /// Constructs an unsaved attachment.
    pub fn new_unsaved<C>(content_type: mime::Mime, content: C) -> Self
    where
        C: Into<Vec<u8>>,
    {
        AttachmentBuilder {
            target: Attachment::Unsaved(UnsavedAttachment {
                content_type: content_type,
                content: content.into(),
            }),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<M> AttachmentBuilder<M> {
    /// Returns the builder's attachment.
    pub fn unwrap(self) -> Attachment {
        self.target
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use super::{AttachmentEncodingInfo, Base64JsonDecodable, Base64JsonEncodable, ContentTypeJsonDecodable,
                SavedAttachmentContent};
    use {base64, serde_json};

    #[test]
    fn attachment_serialize_saved() {

        let attachment = Attachment::Saved(SavedAttachment {
            content_type: mime::TEXT_PLAIN,
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 17,
            content: SavedAttachmentContent::Bytes("This is the attachment.".to_string().into_bytes()),
            encoding_info: None,
        });

        let got = serde_json::to_string(&attachment).unwrap();
        let expected = r#"{"stub":true}"#;

        let got: serde_json::Value = serde_json::from_str(&got).unwrap();
        let expected: serde_json::Value = serde_json::from_str(expected).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn attachment_serialize_unsaved() {

        let content = "This is the attachment.";

        let attachment = Attachment::Unsaved(UnsavedAttachment {
            content_type: mime::TEXT_PLAIN,
            content: content.to_string().into_bytes(),
        });

        let got = serde_json::to_string(&attachment).unwrap();

        let expected = format!(
            "{}{}{}",
            r#"{"content_type":"text/plain","data":""#,
            base64::encode(content.as_bytes()),
            r#""}"#
        );

        let got: serde_json::Value = serde_json::from_str(&got).unwrap();
        let expected: serde_json::Value = serde_json::from_str(&expected).unwrap();

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

        let source = r#"{"content_type":"text/plain",
                         "digest":"md5-iMaiC8wqiFlD2NjLTemvCQ==",
                         "length":5,
                         "revpos":11,
                         "stub":true}"#;

        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn saved_attachment_serialize() {

        let attachment = SavedAttachment {
            content_type: mime::TEXT_PLAIN,
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 17,
            content: SavedAttachmentContent::Bytes("This is the attachment.".to_string().into_bytes()),
            encoding_info: None,
        };

        let got = serde_json::to_string(&attachment).unwrap();
        let expected = r#"{"stub":true}"#;

        let got: serde_json::Value = serde_json::from_str(&got).unwrap();
        let expected: serde_json::Value = serde_json::from_str(expected).unwrap();

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

        let source = r#"{"content_type":"text/plain",
                         "digest":"md5-iMaiC8wqiFlD2NjLTemvCQ==",
                         "length":5,
                         "revpos":11,
                         "stub":true}"#;

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

        let source = r#"{"content_type":"text/plain",
                         "digest":"md5-iMaiC8wqiFlD2NjLTemvCQ==",
                         "encoded_length":25,
                         "encoding":"gzip",
                         "length":5,
                         "revpos":11,
                         "stub":true}"#;

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

        let source = r#"{"content_type":"text/plain",
                         "data":"aGVsbG8=",
                         "digest":"md5-iMaiC8wqiFlD2NjLTemvCQ==",
                         "revpos":11}"#;

        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn saved_attachment_deserialize_nok_missing_content_type() {

        let source = r#"{"digest":"md5-iMaiC8wqiFlD2NjLTemvCQ==",
                         "length":5,
                         "revpos":11,
                         "stub":true}"#;

        match serde_json::from_str::<SavedAttachment>(&source) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }

    #[test]
    fn saved_attachment_deserialize_nok_missing_digest() {

        let source = r#"{"content_type":"text/plain",
                         "length":5,
                         "revpos":11,
                         "stub":true}"#;

        match serde_json::from_str::<SavedAttachment>(&source) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }

    #[test]
    fn saved_attachment_deserialize_nok_missing_revpos() {

        let source = r#"{"content_type":"text/plain",
                         "digest":"md5-iMaiC8wqiFlD2NjLTemvCQ==",
                         "length":5,
                         "stub":true}"#;

        match serde_json::from_str::<SavedAttachment>(&source) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }

    #[test]
    fn unsaved_attachment_serialize() {

        let content = "This is the attachment.";

        let attachment = UnsavedAttachment {
            content_type: mime::TEXT_PLAIN,
            content: content.to_string().into_bytes(),
        };

        let got = serde_json::to_string(&attachment).unwrap();

        let expected = format!(
            "{}{}{}",
            r#"{"content_type":"text/plain","data":""#,
            base64::encode(content.as_bytes()),
            r#""}"#
        );

        let got: serde_json::Value = serde_json::from_str(&got).unwrap();
        let expected: serde_json::Value = serde_json::from_str(&expected).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn base64_json_encodable_serialize() {
        let bytes = "Blah blah blah".to_string().into_bytes();
        let encoded = serde_json::to_string(&Base64JsonEncodable(&bytes)).unwrap();
        assert_eq!(r#""QmxhaCBibGFoIGJsYWg=""#, encoded);
    }

    #[test]
    fn base64_json_encodable_deserialize_ok() {
        let expected = Base64JsonDecodable("hello".to_owned().into_bytes());
        let source = serde_json::Value::String("aGVsbG8=".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn base64_json_encodable_deserialize_nok_bad_base64() {
        let source = serde_json::Value::String("% percent signs are invalid in base64 %".to_string());
        let source = serde_json::to_string(&source).unwrap();
        match serde_json::from_str::<Base64JsonDecodable>(&source) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }

    #[test]
    fn content_type_json_decodable_deserialize_ok() {
        let expected = ContentTypeJsonDecodable(mime::APPLICATION_JSON);
        let source = serde_json::Value::String("application/json".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn content_type_json_decodable_deserialize_nok_bad_mime() {
        let source = serde_json::Value::String("bad mime".to_string());
        let source = serde_json::to_string(&source).unwrap();
        match serde_json::from_str::<ContentTypeJsonDecodable>(&source) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }
}
