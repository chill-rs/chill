use Error;
use mime;
use serde;
use serializable_base64_blob::SerializableBase64Blob;
use serializable_content_type::SerializableContentType;

#[derive(Debug, PartialEq)]
struct AttachmentEncodingInfo {
    encoded_length: u64,
    encoding: String,
}

#[derive(Debug, PartialEq)]
enum AttachmentContent {
    LengthOnly(u64),
    Bytes(Vec<u8>),
}

#[derive(Debug, PartialEq)]
pub struct Attachment {
    content_type: mime::Mime,
    digest: String,
    sequence_number: u64,
    content: AttachmentContent,
    encoding_info: Option<AttachmentEncodingInfo>,
}

impl Attachment {
    pub fn content_type(&self) -> &mime::Mime {
        &self.content_type
    }

    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    pub fn content_length(&self) -> u64 {
        match self.content {
            AttachmentContent::LengthOnly(length) => length,
            AttachmentContent::Bytes(ref bytes) => bytes.len() as u64,
        }
    }

    pub fn content_bytes(&self) -> Option<&[u8]> {
        match self.content {
            AttachmentContent::LengthOnly(..) => None,
            AttachmentContent::Bytes(ref bytes) => Some(&bytes),
        }
    }
}

impl serde::Deserialize for Attachment {
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
            type Value = Attachment;

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
                    (Some(SerializableBase64Blob(data)), None) => AttachmentContent::Bytes(data),
                    (None, Some(length)) => AttachmentContent::LengthOnly(length),
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

                Ok(Attachment {
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
        deserializer.deserialize_struct("Attachment", FIELDS, Visitor)
    }
}

#[cfg(test)]
pub struct AttachmentBuilder {
    target_attachment: Attachment,
}

#[cfg(test)]
impl AttachmentBuilder {
    pub fn new_stub(content_type: mime::Mime,
                    digest: String,
                    sequence_number: u64,
                    content_length: u64)
                    -> Self {
        AttachmentBuilder {
            target_attachment: Attachment {
                content_type: content_type,
                digest: digest,
                sequence_number: sequence_number,
                content: AttachmentContent::LengthOnly(content_length),
                encoding_info: None,
            },
        }
    }

    pub fn unwrap(self) -> Attachment {
        self.target_attachment
    }
}

#[cfg(test)]
mod tests {

    use serde_json;
    use super::{AttachmentBuilder, AttachmentContent, AttachmentEncodingInfo, Attachment};

    #[test]
    fn deserialize_ok_as_stub() {

        let expected = Attachment {
            content_type: "text/plain".parse().unwrap(),
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 11,
            content: AttachmentContent::LengthOnly(5),
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
    fn deserialize_ok_as_stub_with_encoding_info() {

        let expected = Attachment {
            content_type: "text/plain".parse().unwrap(),
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 11,
            content: AttachmentContent::LengthOnly(5),
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
    fn deserialize_ok_with_content_body() {

        let expected = Attachment {
            content_type: "text/plain".parse().unwrap(),
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 11,
            content: AttachmentContent::Bytes("hello".to_string().into_bytes()),
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
    fn deserialize_nok_missing_content_type() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("digest", "md5-iMaiC8wqiFlD2NjLTemvCQ==")
                         .insert("length", 5)
                         .insert("revpos", 11)
                         .insert("stub", true)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<Attachment>(&source);
        expect_json_error_missing_field!(got, "content_type");
    }


    #[test]
    fn deserialize_nok_missing_digest() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("content_type", "text/plain")
                         .insert("length", 5)
                         .insert("revpos", 11)
                         .insert("stub", true)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<Attachment>(&source);
        expect_json_error_missing_field!(got, "digest");
    }

    #[test]
    fn deserialize_nok_missing_revpos() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("content_type", "text/plain")
                         .insert("digest", "md5-iMaiC8wqiFlD2NjLTemvCQ==")
                         .insert("length", 5)
                         .insert("stub", true)
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<Attachment>(&source);
        expect_json_error_missing_field!(got, "revpos");
    }

    #[test]
    fn builder_as_stub() {

        let expected = Attachment {
            content_type: mime!(Text / Plain),
            digest: "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
            sequence_number: 11,
            content: AttachmentContent::LengthOnly(5),
            encoding_info: None,
        };

        let got = AttachmentBuilder::new_stub(mime!(Text / Plain),
                                              "md5-iMaiC8wqiFlD2NjLTemvCQ==".to_string(),
                                              11,
                                              5)
                      .unwrap();
        assert_eq!(expected, got);
    }
}
