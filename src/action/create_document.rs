use DocumentId;
use document::WriteDocumentResponse;
use Error;
use IntoDatabasePath;
use Revision;
use serde;
use serde_json;
use transport::{RequestOptions, Response, StatusCode, Transport};

pub struct CreateDocument<'a, C, P, T>
    where C: serde::Serialize + 'a,
          P: IntoDatabasePath,
          T: Transport + 'a
{
    transport: &'a T,
    db_path: P,
    content: &'a C,
    doc_id: Option<&'a DocumentId>,
}

impl<'a, C, P, T> CreateDocument<'a, C, P, T>
    where C: serde::Serialize + 'a,
          P: IntoDatabasePath,
          T: Transport + 'a
{
    #[doc(hidden)]
    pub fn new(transport: &'a T, db_path: P, content: &'a C) -> Self {
        CreateDocument {
            transport: transport,
            db_path: db_path,
            content: content,
            doc_id: None,
        }
    }

    pub fn with_document_id(mut self, doc_id: &'a DocumentId) -> Self {
        self.doc_id = Some(doc_id);
        self
    }

    pub fn run(self) -> Result<(DocumentId, Revision), Error> {

        let db_name = try!(self.db_path.into_database_path());

        let body = {
            let mut doc = serde_json::to_value(self.content);

            match doc {
                serde_json::Value::Object(ref mut fields) => {
                    for doc_id in self.doc_id {
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
                                .post(db_name.iter(),
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
}

#[cfg(test)]
mod tests {

    use DocumentId;
    use Error;
    use Revision;
    use serde_json;
    use super::*;
    use transport::{MockRequestMatcher, MockResponse, MockTransport, StatusCode};

    #[test]
    fn create_document_ok_with_default_options() {

        let transport = MockTransport::new();
        transport.push_response(MockResponse::new(StatusCode::Created).build_json_body(|x| {
            x.insert("ok", true)
             .insert("id", "17a0e088c69e0a99be6d6159b4000563")
             .insert("rev", "1-967a00dff5e02add41819138abb3284d")
        }));

        let doc_content = serde_json::builder::ObjectBuilder::new()
                              .insert("field_1", 42)
                              .insert("field_2", "hello")
                              .unwrap();

        let (doc_id, revision) = CreateDocument::new(&transport, "/database_name", &doc_content)
                                     .run()
                                     .unwrap();

        let expected = (DocumentId::from("17a0e088c69e0a99be6d6159b4000563"),
                        Revision::parse("1-967a00dff5e02add41819138abb3284d").unwrap());
        assert_eq!(expected, (doc_id, revision));

        let expected = MockRequestMatcher::new().post(&["database_name"], |x| {
            x.with_accept_json()
             .build_json_body(|x| {
                 x.insert("field_1", 42)
                  .insert("field_2", "hello")
             })
        });
        assert_eq!(expected, transport.extract_requests());
    }

    #[test]
    fn create_document_ok_with_document_id() {

        let transport = MockTransport::new();
        transport.push_response(MockResponse::new(StatusCode::Created).build_json_body(|x| {
            x.insert("ok", true)
             .insert("id", "document_id")
             .insert("rev", "1-967a00dff5e02add41819138abb3284d")
        }));

        let doc_content = serde_json::builder::ObjectBuilder::new()
                              .insert("field_1", 42)
                              .insert("field_2", "hello")
                              .unwrap();

        let (doc_id, revision) = CreateDocument::new(&transport, "/database_name", &doc_content)
                                     .with_document_id(&DocumentId::from("document_id"))
                                     .run()
                                     .unwrap();

        let expected = (DocumentId::from("document_id"),
                        Revision::parse("1-967a00dff5e02add41819138abb3284d").unwrap());
        assert_eq!(expected, (doc_id, revision));

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
        assert_eq!(expected, transport.extract_requests());
    }

    #[test]
    fn create_document_nok_document_conflict() {

        let transport = MockTransport::new();
        let error = "conflict";
        let reason = "Document update conflict.";
        transport.push_response(MockResponse::new(StatusCode::Conflict).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        let doc_content = serde_json::builder::ObjectBuilder::new().unwrap();

        match CreateDocument::new(&transport, "/database_name", &doc_content).run() {
            Err(Error::DocumentConflict(ref error_response)) if error == error_response.error() &&
                                                                reason ==
                                                                error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn create_document_nok_unauthorized() {

        let transport = MockTransport::new();
        let error = "unauthorized";
        let reason = "Authentication required.";
        transport.push_response(MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        let doc_content = serde_json::builder::ObjectBuilder::new().unwrap();

        match CreateDocument::new(&transport, "/database_name", &doc_content).run() {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
