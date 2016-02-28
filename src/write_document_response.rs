use DocumentId;
use Revision;
use serde;

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

    use serde_json;
    use super::WriteDocumentResponse;

    #[test]
    fn deserialize_ok_with_all_fields() {
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
    fn deserialize_nok_missing_id_field() {
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("ok", true)
                         .insert("rev", "1-12345678123456781234567812345678")
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<WriteDocumentResponse>(&source);
        expect_json_error_missing_field!(got, "id");
    }

    #[test]
    fn deserialize_nok_missing_ok_field() {
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("id", "foo")
                         .insert("rev", "1-12345678123456781234567812345678")
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<WriteDocumentResponse>(&source);
        expect_json_error_missing_field!(got, "ok");
    }

    #[test]
    fn deserialize_nok_missing_rev_field() {
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("id", "foo")
                         .insert("ok", true)
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<WriteDocumentResponse>(&source);
        expect_json_error_missing_field!(got, "rev");
    }
}
