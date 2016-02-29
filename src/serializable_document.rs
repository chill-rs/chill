use DocumentId;
use Revision;
use serde;
use serde_json;
use attachment::Attachment;
use std;

#[derive(Debug, PartialEq)]
pub struct SerializableDocument {
    pub id: DocumentId,
    pub revision: Revision,
    pub attachments: std::collections::HashMap<String, Attachment>,
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

#[cfg(test)]
mod tests {

    use DocumentId;
    use serde_json;
    use attachment::AttachmentBuilder;
    use std;
    use super::SerializableDocument;

    #[test]
    fn deserialize_ok_as_minimum() {

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
    fn deserialize_ok_as_deleted() {

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
    fn deserialize_ok_with_content() {

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
    fn deserialize_ok_with_attachments() {

        let expected = SerializableDocument {
            id: DocumentId::from("document_id"),
            revision: "42-1234567890abcdef1234567890abcdef".parse().unwrap(),
            attachments: {
                let mut map = std::collections::HashMap::new();
                map.insert(String::from("attachment_1"),
                           AttachmentBuilder::new_stub(mime!(Application / WwwFormUrlEncoded),
                                                       "md5-XNdWXQ0FO9vPx7skS0GuYA==".to_string(),
                                                       23,
                                                       517)
                               .unwrap());
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
    fn deserialize_nok_missing_id() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_rev", "42-1234567890abcdef1234567890abcdef")
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SerializableDocument>(&source);
        expect_json_error_missing_field!(got, "_id");
    }

    #[test]
    fn deserialize_nok_missing_rev() {

        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("_id", "document_id")
                         .unwrap();

        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SerializableDocument>(&source);
        expect_json_error_missing_field!(got, "_rev");
    }
}
