use BasicDocument;
use CreateDocumentOptions;
use DatabaseName;
use DocumentId;
use document::WriteDocumentResponse;
use Error;
use hyper;
use ReadDocumentOptions;
use Revision;
use serde;
use serde_json;
use std;
use transport::{HyperTransport, RequestBuilder, Transport};

pub type Database = BasicDatabase<HyperTransport>;

#[derive(Debug)]
pub struct BasicDatabase<T> {
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

        let doc = {
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

        let request = RequestBuilder::new(hyper::method::Method::Post,
                                          vec![String::from(self.db_name.clone())])
                          .with_accept_json()
                          .with_json_body(&doc)
                          .unwrap();

        let response = try!(self.transport.send(request));

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
                            -> Result<BasicDocument<T>, Error>
        where D: Into<DocumentId>
    {
        use hyper::status::StatusCode;

        let request = RequestBuilder::new(hyper::Get,
                                          vec![String::from(self.db_name.clone()),
                                               String::from(doc_id.into())])
                          .with_accept_json()
                          .unwrap();

        let response = try!(self.transport.send(request));

        match response.status_code() {

            StatusCode::Ok => {
                let doc = try!(response.decode_json_body());
                let doc = BasicDocument::from_serializable_document(&self.transport,
                                                                    &self.db_name,
                                                                    doc);
                Ok(doc)
            }

            StatusCode::NotFound => Err(Error::not_found(response)),
            StatusCode::Unauthorized => Err(Error::unauthorized(response)),
            _ => Err(Error::server_response(response)),
        }
    }
}

#[cfg(test)]
mod tests {

    use CreateDocumentOptions;
    use DatabaseName;
    use Error;
    use hyper;
    use serde_json;
    use std;
    use super::BasicDatabase;
    use transport::{MockTransport, Request, RequestBuilder, Response, ResponseBuilder};

    type MockDatabase = BasicDatabase<MockTransport>;

    impl MockDatabase {
        fn extract_requests(&self) -> Vec<Request> {
            self.transport.extract_requests()
        }

        fn push_response(&self, response: Response) {
            self.transport.push_response(response)
        }
    }

    fn new_mock_database<D: Into<DatabaseName>>(db_name: D) -> MockDatabase {
        let transport = std::sync::Arc::new(MockTransport::new());
        MockDatabase::new(transport, db_name.into())
    }

    #[test]
    fn create_document_ok_basic() {

        let db = new_mock_database("database_name");
        db.push_response(ResponseBuilder::new(hyper::status::StatusCode::Created)
                             .with_json_body_builder(|x| {
                                 x.insert("ok", true)
                                  .insert("id", "17a0e088c69e0a99be6d6159b4000563")
                                  .insert("rev", "1-967a00dff5e02add41819138abb3284d")
                             })
                             .unwrap());

        let doc_content = serde_json::builder::ObjectBuilder::new()
                              .insert("field_1", 42)
                              .insert("field_2", 17)
                              .unwrap();

        db.create_document(&doc_content, Default::default()).unwrap();

        let expected_requests = {
            vec![RequestBuilder::new(hyper::Post, vec![String::from("database_name")])
                     .with_accept_json()
                     .with_json_body_builder(|x| {
                         x.insert("field_1", 42)
                          .insert("field_2", 17)
                     })
                     .unwrap()]
        };

        assert_eq!(expected_requests, db.extract_requests());
    }

    #[test]
    fn create_document_ok_with_document_id() {

        let db = new_mock_database("database_name");
        db.push_response(ResponseBuilder::new(hyper::status::StatusCode::Created)
                             .with_json_body_builder(|x| {
                                 x.insert("ok", true)
                                  .insert("id", "document_id")
                                  .insert("rev", "1-967a00dff5e02add41819138abb3284d")
                             })
                             .unwrap());

        let doc_content = serde_json::builder::ObjectBuilder::new()
                              .insert("field_1", 42)
                              .insert("field_2", 17)
                              .unwrap();

        db.create_document(&doc_content,
                           CreateDocumentOptions::new().with_document_id("document_id"))
          .unwrap();

        let expected_requests = {
            vec![RequestBuilder::new(hyper::Post, vec![String::from("database_name")])
                     .with_accept_json()
                     .with_json_body_builder(|x| {
                         x.insert("_id", "document_id")
                          .insert("field_1", 42)
                          .insert("field_2", 17)
                     })
                     .unwrap()]
        };

        assert_eq!(expected_requests, db.extract_requests());
    }

    #[test]
    fn create_document_nok_document_conflict() {

        let db = new_mock_database("database_name");
        let error = "conflict";
        let reason = "Document update conflict.";
        db.push_response(ResponseBuilder::new(hyper::status::StatusCode::Conflict)
                             .with_json_body_builder(|x| {
                                 x.insert("error", error)
                                  .insert("reason", reason)
                             })
                             .unwrap());

        let doc_content = serde_json::builder::ObjectBuilder::new()
                              .insert("field_1", 42)
                              .insert("field_2", 17)
                              .unwrap();

        match db.create_document(&doc_content,
                                 CreateDocumentOptions::new().with_document_id("document_id")) {
            Err(Error::DocumentConflict(ref error_response)) if error == error_response.error() &&
                                                                reason ==
                                                                error_response.reason() => (),
            e @ _ => unexpected_result!(e),
        }

        let expected_requests = {
            vec![RequestBuilder::new(hyper::Post, vec![String::from("database_name")])
                     .with_accept_json()
                     .with_json_body_builder(|x| {
                         x.insert("_id", "document_id")
                          .insert("field_1", 42)
                          .insert("field_2", 17)
                     })
                     .unwrap()]
        };

        assert_eq!(expected_requests, db.extract_requests());
    }

    #[test]
    fn create_document_nok_unauthorized() {

        let db = new_mock_database("database_name");
        let error = "unauthorized";
        let reason = "Authentication required.";
        db.push_response(ResponseBuilder::new(hyper::status::StatusCode::Unauthorized)
                             .with_json_body_builder(|x| {
                                 x.insert("error", error)
                                  .insert("reason", reason)
                             })
                             .unwrap());

        let doc_content = serde_json::builder::ObjectBuilder::new()
                              .insert("field_1", 42)
                              .insert("field_2", 17)
                              .unwrap();

        match db.create_document(&doc_content, Default::default()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            e @ _ => unexpected_result!(e),
        }

        let expected_requests = {
            vec![RequestBuilder::new(hyper::Post, vec![String::from("database_name")])
                     .with_accept_json()
                     .with_json_body_builder(|x| {
                         x.insert("field_1", 42)
                          .insert("field_2", 17)
                     })
                     .unwrap()]
        };

        assert_eq!(expected_requests, db.extract_requests());
    }

    #[test]
    fn read_document_ok_basic() {

        let db = new_mock_database("database_name");
        db.push_response(ResponseBuilder::new(hyper::status::StatusCode::Ok)
                             .with_json_body_builder(|x| {
                                 x.insert("_id", "document_id")
                                  .insert("_rev", "1-967a00dff5e02add41819138abb3284d")
                                  .insert("field_1", 42)
                                  .insert("field_2", "foo")
                             })
                             .unwrap());

        db.read_document("document_id", Default::default()).unwrap();

        let expected_requests = {
            vec![RequestBuilder::new(hyper::Get,
                                     vec![String::from("database_name"),
                                          String::from("document_id")])
                     .with_accept_json()
                     .unwrap()]
        };

        assert_eq!(expected_requests, db.extract_requests());
    }

    #[test]
    fn read_document_nok_not_found() {

        let db = new_mock_database("database_name");
        let error = "not_found";
        let reason = "missing";
        db.push_response(ResponseBuilder::new(hyper::status::StatusCode::Unauthorized)
                             .with_json_body_builder(|x| {
                                 x.insert("error", error)
                                  .insert("reason", reason)
                             })
                             .unwrap());

        match db.read_document("document_id", Default::default()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            e @ _ => unexpected_result!(e),
        }

        let expected_requests = {
            vec![RequestBuilder::new(hyper::Get,
                                     vec![String::from("database_name"),
                                          String::from("document_id")])
                     .with_accept_json()
                     .unwrap()]
        };

        assert_eq!(expected_requests, db.extract_requests());
    }

    #[test]
    fn read_document_nok_unauthorized() {

        let db = new_mock_database("database_name");
        let error = "unauthorized";
        let reason = "Authentication required.";
        db.push_response(ResponseBuilder::new(hyper::status::StatusCode::Unauthorized)
                             .with_json_body_builder(|x| {
                                 x.insert("error", error)
                                  .insert("reason", reason)
                             })
                             .unwrap());

        match db.read_document("document_id", Default::default()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            e @ _ => unexpected_result!(e),
        }

        let expected_requests = {
            vec![RequestBuilder::new(hyper::Get,
                                     vec![String::from("database_name"),
                                          String::from("document_id")])
                     .with_accept_json()
                     .unwrap()]
        };

        assert_eq!(expected_requests, db.extract_requests());
    }
}
