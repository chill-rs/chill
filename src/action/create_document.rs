use DatabaseName;
use DocumentId;
use Error;
use hyper;
use Revision;
use serde;
use serde_json;
use std;
use transport::{Action, Request, RequestMaker, Response, Transport};
use write_document_response::WriteDocumentResponse;

struct ActionState;

pub struct CreateDocument<'a, C: serde::Serialize + 'a> {
    transport: &'a std::sync::Arc<Transport>,
    doc_content: &'a C,
    db_name: DatabaseName,
    doc_id: Option<DocumentId>,
}

impl<'a, C> CreateDocument<'a, C>
    where C: serde::Serialize + 'a
{
    #[doc(hidden)]
    pub fn new(transport: &'a std::sync::Arc<Transport>,
               db_name: DatabaseName,
               doc_content: &'a C)
               -> Self {
        CreateDocument {
            doc_content: doc_content,
            db_name: db_name,
            doc_id: None,
            transport: transport,
        }
    }

    pub fn set_document_id<D>(mut self, doc_id: D) -> Self
        where D: Into<DocumentId>
    {
        self.doc_id = Some(doc_id.into());
        self
    }

    pub fn run(self) -> Result<(DocumentId, Revision), Error> {
        self.transport.run_action(self)
    }
}

impl<'a, C> Action for CreateDocument<'a, C>
    where C: serde::Serialize + 'a
{
    type Output = (DocumentId, Revision);
    type State = ActionState;

    fn create_request<R>(self, request_maker: R) -> Result<(R::Request, Self::State), Error>
        where R: RequestMaker
    {
        let doc_content = {
            let mut doc_content = serde_json::to_value(self.doc_content);

            // Need to remove all meta JSON fields (e.g., "_id") in the
            // handle_response method.

            match doc_content {
                serde_json::Value::Object(ref mut fields) => {
                    if let Some(doc_id) = self.doc_id {
                        fields.insert("_id".to_owned(), serde_json::to_value(&doc_id));
                    }
                }
                _ => {
                    return Err(Error::ContentNotAnObject);
                }
            };

            doc_content
        };

        let body = try!(serde_json::to_vec(&doc_content)
                            .map_err(|e| Error::JsonDecode { cause: e }));

        let request = request_maker.make_request(hyper::method::Method::Post,
                                                 vec![self.db_name.clone()].into_iter())
                                   .set_content_type_json()
                                   .set_body(body);

        Ok((request, ActionState))
    }

    fn handle_response<R>(response: R, _state: Self::State) -> Result<Self::Output, Error>
        where R: Response
    {
        use hyper::status::StatusCode;

        match response.status_code() {

            StatusCode::Created => {
                let body: WriteDocumentResponse = try!(response.json_decode_content());
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
    use hyper;
    use Revision;
    use serde_json;
    use std;
    use super::{ActionState, CreateDocument};
    use transport::{Action, Request, StubRequest, StubRequestMaker, StubResponse, Transport};

    #[test]
    fn create_request_without_doc_id() {

        let content = serde_json::builder::ObjectBuilder::new()
                          .insert("field", 42)
                          .unwrap();

        let expected_request = StubRequest::new(hyper::method::Method::Post, &["foo"])
                                   .set_content_type_json()
                                   .set_body(serde_json::to_vec(&content).unwrap());

        let transport = std::sync::Arc::new(Transport::new_stub());
        let action = CreateDocument::new(&transport, "foo".to_owned(), &content);
        let (got_request, _) = action.create_request(StubRequestMaker::new())
                                     .unwrap();
        assert_eq!(expected_request, got_request);
    }

    #[test]
    fn create_request_with_doc_id() {

        let content = serde_json::builder::ObjectBuilder::new()
                          .insert("field", 42)
                          .unwrap();

        let expected_body = serde_json::builder::ObjectBuilder::new()
                                .insert("field", 42)
                                .insert("_id", "bar")
                                .unwrap();

        let expected_request = StubRequest::new(hyper::method::Method::Post, &["foo"])
                                   .set_content_type_json()
                                   .set_body(serde_json::to_vec(&expected_body).unwrap());

        let transport = std::sync::Arc::new(Transport::new_stub());
        let action = CreateDocument::new(&transport, "foo".to_owned(), &content)
                         .set_document_id("bar");
        let (got_request, _) = action.create_request(StubRequestMaker::new())
                                     .unwrap();
        assert_eq!(expected_request, got_request);
    }

    #[test]
    fn handle_response_ok_created() {

        let response = StubResponse::new(hyper::status::StatusCode::Created)
                           .build_json_content(|builder| {
                               builder.insert("ok", true)
                                      .insert("id", "bar")
                                      .insert("rev", "1-1234567890abcdef1234567890abcdef")
                           });

        let expected_doc_id = DocumentId::from("bar");
        let expected_rev = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();

        let (got_doc_id, got_rev) = CreateDocument::<()>::handle_response(response, ActionState)
                                        .unwrap();

        assert_eq!(expected_doc_id, got_doc_id);
        assert_eq!(expected_rev, got_rev);
    }

    #[test]
    fn handle_response_nok_unauthorized() {

        let error = "unauthorized";
        let reason = "Authentication required.";
        let response = StubResponse::new(hyper::status::StatusCode::Unauthorized)
                           .set_error_content(error, reason);

        let got = CreateDocument::<()>::handle_response(response, ActionState);
        expect_error_unauthorized!(got, error, reason);
    }

    #[test]
    fn handle_response_nok_document_conflict() {

        let error = "conflict";
        let reason = "Document update conflict.";
        let response = StubResponse::new(hyper::status::StatusCode::Conflict)
                           .set_error_content(error, reason);

        let got = CreateDocument::<()>::handle_response(response, ActionState);
        expect_error_document_conflict!(got, error, reason);
    }

}
