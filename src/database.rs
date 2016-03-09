use CreateDocumentOptions;
use DatabaseName;
use Document;
use DocumentId;
use Error;
use ReadDocumentOptions;
use Revision;
use serde;
use serde_json;
use std;
use transport::{HyperTransport, RequestOptions, Response, StatusCode, Transport};
use UpdateDocumentOptions;

pub type Database = BasicDatabase<HyperTransport>;

#[derive(Debug)]
pub struct BasicDatabase<T: Transport> {
    transport: std::sync::Arc<T>,
    db_name: DatabaseName,
}

impl<T: Transport> BasicDatabase<T> {
    #[doc(hidden)]
    pub fn new(transport: std::sync::Arc<T>, db_name: DatabaseName) -> Self {
        BasicDatabase {
            db_name: db_name,
            transport: transport,
        }
    }

    pub fn name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn create_document<C>(&self,
                              content: &C,
                              options: CreateDocumentOptions)
                              -> Result<(DocumentId, Revision), Error>
        where C: serde::Serialize
    {
        use hyper::status::StatusCode;

        let body = {
            let mut doc = serde_json::to_value(content);

            match doc {
                serde_json::Value::Object(ref mut fields) => {
                    for doc_id in options.document_id() {
                        fields.insert(String::from("_id"), serde_json::to_value(&doc_id));
                    }
                }
                _ => {
                    return Err(Error::ContentNotAnObject);
                }
            }

            doc
        };

        let response = try!(self.transport
                                .post(&[&self.db_name],
                                      RequestOptions::new()
                                          .with_accept_json()
                                          .with_json_body(&body)));

        match response.status_code() {
            StatusCode::Created => {
                let body: WriteDocumentResponse = try!(response.decode_json_body());
                Ok((body.doc_id, body.revision))
            }

            StatusCode::Conflict => Err(Error::document_conflict(response)),
            StatusCode::Unauthorized => Err(Error::unauthorized(response)),
            _ => Err(Error::server_response(response)),
        }
    }

    pub fn read_document<D>(&self,
                            doc_id: D,
                            _options: ReadDocumentOptions)
                            -> Result<Document, Error>
        where D: Into<DocumentId>
    {
        // FIXME: Eliminate this temporary.
        let doc_id = String::from(doc_id.into());

        let response = try!(self.transport
                                .get(&[&self.db_name, &doc_id],
                                     RequestOptions::new().with_accept_json()));

        match response.status_code() {
            StatusCode::Ok => response.decode_json_body(),
            StatusCode::NotFound => Err(Error::not_found(response)),
            StatusCode::Unauthorized => Err(Error::unauthorized(response)),
            _ => Err(Error::server_response(response)),
        }
    }

    pub fn update_document(&self,
                           doc: &Document,
                           _options: UpdateDocumentOptions)
                           -> Result<Revision, Error> {

        // FIXME: Eliminate this temporary.
        let doc_id = String::from(doc.id().clone());

        // FIXME: Serialize the document.
        let response = try!(self.transport
                                .put(&[&self.db_name, &doc_id],
                                     RequestOptions::new()
                                         .with_accept_json()
                                         .with_revision_query(&doc.revision())
                                         .with_json_body(doc)));

        match response.status_code() {

            StatusCode::Created => {
                let body: WriteDocumentResponse = try!(response.decode_json_body());
                Ok(body.revision)
            }

            StatusCode::Conflict => Err(Error::document_conflict(response)),
            StatusCode::Unauthorized => Err(Error::unauthorized(response)),
            _ => Err(Error::server_response(response)),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct WriteDocumentResponse {
    ok: bool,
    doc_id: DocumentId,
    revision: Revision,
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

    use CreateDocumentOptions;
    use DatabaseName;
    use DocumentId;
    use document::DocumentBuilder;
    use Error;
    use Revision;
    use serde_json;
    use std;
    use super::{BasicDatabase, WriteDocumentResponse};
    use transport::{MockRequestMatcher, MockResponse, MockTransport, StatusCode};

    fn new_mock_database<D: Into<DatabaseName>>(db_name: D) -> BasicDatabase<MockTransport> {
        BasicDatabase {
            transport: std::sync::Arc::new(MockTransport::new()),
            db_name: db_name.into(),
        }
    }

    #[test]
    fn create_document_ok_with_default_options() {

        let db = new_mock_database("database_name");
        db.transport.push_response(MockResponse::new(StatusCode::Created).build_json_body(|x| {
            x.insert("ok", true)
             .insert("id", "17a0e088c69e0a99be6d6159b4000563")
             .insert("rev", "1-967a00dff5e02add41819138abb3284d")
        }));

        let doc_content = serde_json::builder::ObjectBuilder::new()
                              .insert("field_1", 42)
                              .insert("field_2", "hello")
                              .unwrap();

        let (doc_id, revision) = db.create_document(&doc_content, Default::default())
                                   .unwrap();

        assert_eq!(DocumentId::from("17a0e088c69e0a99be6d6159b4000563"), doc_id);
        assert_eq!(Revision::parse("1-967a00dff5e02add41819138abb3284d").unwrap(),
                   revision);

        let expected = MockRequestMatcher::new().post(&["database_name"], |x| {
            x.with_accept_json()
             .build_json_body(|x| {
                 x.insert("field_1", 42)
                  .insert("field_2", "hello")
             })
        });
        assert_eq!(expected, db.transport.extract_requests());
    }

    #[test]
    fn create_document_ok_with_document_id() {

        let db = new_mock_database("database_name");
        db.transport.push_response(MockResponse::new(StatusCode::Created).build_json_body(|x| {
            x.insert("ok", true)
             .insert("id", "document_id")
             .insert("rev", "1-967a00dff5e02add41819138abb3284d")
        }));

        let doc_content = serde_json::builder::ObjectBuilder::new()
                              .insert("field_1", 42)
                              .insert("field_2", "hello")
                              .unwrap();

        let (doc_id, revision) = db.create_document(&doc_content,
                                                    CreateDocumentOptions::new()
                                                        .with_document_id("document_id"))
                                   .unwrap();

        assert_eq!(DocumentId::from("document_id"), doc_id);
        assert_eq!(Revision::parse("1-967a00dff5e02add41819138abb3284d").unwrap(),
                   revision);

        let expected = {
            MockRequestMatcher::new().post(&["database_name"], |x| {
                x.with_accept_json()
                 .build_json_body(|x| {
                     x.insert("_id", "document_id")
                      .insert("field_1", 42)
                      .insert("field_2", "hello")
                 })
            })
        };
        assert_eq!(expected, db.transport.extract_requests());
    }

    #[test]
    fn create_document_nok_document_conflict() {

        let db = new_mock_database("database_name");
        let error = "conflict";
        let reason = "Document update conflict.";
        db.transport.push_response(MockResponse::new(StatusCode::Conflict).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        let doc_content = serde_json::builder::ObjectBuilder::new().unwrap();

        match db.create_document(&doc_content,
                                 CreateDocumentOptions::new().with_document_id("document_id")) {
            Err(Error::DocumentConflict(ref error_response)) if error == error_response.error() &&
                                                                reason ==
                                                                error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn create_document_nok_unauthorized() {

        let db = new_mock_database("database_name");
        let error = "unauthorized";
        let reason = "Authentication required.";
        db.transport
          .push_response(MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
              x.insert("error", error)
               .insert("reason", reason)
          }));

        let doc_content = serde_json::builder::ObjectBuilder::new().unwrap();

        match db.create_document(&doc_content, Default::default()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn read_document_ok_with_default_options() {

        let db = new_mock_database("database_name");
        db.transport.push_response(MockResponse::new(StatusCode::Ok).build_json_body(|x| {
            x.insert("_id", "document_id")
             .insert("_rev", "1-967a00dff5e02add41819138abb3284d")
             .insert("field_1", 42)
             .insert("field_2", "hello")
        }));

        let expected = DocumentBuilder::new("document_id",
                                            Revision::parse("1-967a00dff5e02add41819138abb3284d")
                                                .unwrap())
                           .build_content(|x| {
                               x.insert("field_1", 42)
                                .insert("field_2", "hello")
                           })
                           .unwrap();

        let doc = db.read_document("document_id", Default::default()).unwrap();
        assert_eq!(expected, doc);

        let expected = {
            MockRequestMatcher::new()
                .get(&["database_name", "document_id"], |x| x.with_accept_json())
        };
        assert_eq!(expected, db.transport.extract_requests());
    }

    #[test]
    fn read_document_nok_not_found() {

        let db = new_mock_database("database_name");
        let error = "not_found";
        let reason = "missing";
        db.transport.push_response(MockResponse::new(StatusCode::NotFound).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        match db.read_document("document_id", Default::default()) {
            Err(Error::NotFound(ref error_response)) if error == error_response.error() &&
                                                        reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn read_document_nok_unauthorized() {

        let db = new_mock_database("database_name");
        let error = "unauthorized";
        let reason = "Authentication required.";
        db.transport
          .push_response(MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
              x.insert("error", error)
               .insert("reason", reason)
          }));

        match db.read_document("document_id", Default::default()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn update_document_ok_basic() {

        let db = new_mock_database("database_name");

        let original_revision: Revision = "1-1234567890abcdef1234567890abcdef".parse().unwrap();

        let doc = DocumentBuilder::new("document_id", original_revision.clone())
                      .build_content(|x| {
                          x.insert("field_1", 42)
                           .insert("field_2", "hello")
                      })
                      .unwrap();

        let new_revision: Revision = "2-fedcba0987654321fedcba0987654321".parse().unwrap();

        db.transport.push_response(MockResponse::new(StatusCode::Created).build_json_body(|x| {
            x.insert("ok", true)
             .insert("id", "document_id")
             .insert("rev", new_revision.to_string())
        }));

        let got = db.update_document(&doc, Default::default()).unwrap();
        assert_eq!(new_revision, got);

        let expected = {
            MockRequestMatcher::new().put(&["database_name", "document_id"], |x| {
                x.with_accept_json()
                 .with_revision_query(&original_revision)
                 .build_json_body(|x| {
                     x.insert("field_1", 42)
                      .insert("field_2", "hello")
                 })
            })
        };

        assert_eq!(expected, db.transport.extract_requests());
    }

    #[test]
    fn update_document_nok_document_conflict() {

        let db = new_mock_database("database_name");
        let error = "conflict";
        let reason = "Document update conflict.";
        db.transport
          .push_response(MockResponse::new(StatusCode::Conflict).build_json_body(|x| {
              x.insert("error", error)
               .insert("reason", reason)
          }));

        let doc = DocumentBuilder::new("document_id",
                                       Revision::parse("42-1234567890abcdef1234567890abcdef")
                                           .unwrap())
                      .unwrap();

        match db.update_document(&doc, Default::default()) {
            Err(Error::DocumentConflict(ref error_response)) if error == error_response.error() &&
                                                                reason ==
                                                                error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn update_document_nok_unauthorized() {

        let db = new_mock_database("database_name");
        let error = "unauthorized";
        let reason = "Authentication required.";
        db.transport
          .push_response(MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
              x.insert("error", error)
               .insert("reason", reason)
          }));

        let doc = DocumentBuilder::new("document_id",
                                       Revision::parse("42-1234567890abcdef1234567890abcdef")
                                           .unwrap())
                      .unwrap();

        match db.update_document(&doc, Default::default()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
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
