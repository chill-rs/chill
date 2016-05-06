use {DatabasePath, DocumentId, Error, IntoDatabasePath, Revision};
use document::WriteDocumentResponse;
use {serde, serde_json};
use transport::{Action, RequestOptions, Response, StatusCode, Transport};
use transport::production::HyperTransport;

pub struct CreateDocument<'a, T, C>
    where C: serde::Serialize + 'a,
          T: Transport + 'a
{
    transport: &'a T,
    db_path: DatabasePath,
    content: &'a C,
    doc_id: Option<DocumentId>,
}

impl<'a, C, T> CreateDocument<'a, T, C>
    where C: serde::Serialize + 'a,
          T: Transport + 'a
{
    #[doc(hidden)]
    pub fn new<P>(transport: &'a T, db_path: P, content: &'a C) -> Result<Self, Error>
        where P: IntoDatabasePath
    {
        Ok(CreateDocument {
            transport: transport,
            db_path: try!(db_path.into_database_path()),
            content: content,
            doc_id: None,
        })
    }

    pub fn with_document_id<D>(mut self, doc_id: D) -> Self
        where D: Into<DocumentId>
    {
        self.doc_id = Some(doc_id.into());
        self
    }
}

impl<'a, C> CreateDocument<'a, HyperTransport, C>
    where C: serde::Serialize + 'a
{
    pub fn run(self) -> Result<(DocumentId, Revision), Error> {
        self.transport.exec_sync(self)
    }
}

impl<'a, C, T> Action<T> for CreateDocument<'a, T, C>
    where C: serde::Serialize + 'a,
          T: Transport + 'a
{
    type Output = (DocumentId, Revision);
    type State = ();

    fn make_request(&mut self) -> Result<(T::Request, Self::State), Error> {

        let body = {
            let mut doc = serde_json::to_value(self.content);

            match doc {
                serde_json::Value::Object(ref mut fields) => {
                    if let Some(ref doc_id) = self.doc_id {
                        fields.insert(String::from("_id"), serde_json::to_value(doc_id));
                    }
                }
                _ => {
                    return Err(Error::ContentNotAnObject);
                }
            }

            doc
        };

        let options = RequestOptions::new().with_accept_json().with_json_body(&body);
        let request = try!(self.transport.post(self.db_path.iter(), options));
        Ok((request, ()))
    }

    fn take_response<R: Response>(response: R, _state: Self::State) -> Result<Self::Output, Error> {
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
    use super::CreateDocument;
    use transport::{Action, RequestOptions, StatusCode, Transport};
    use transport::testing::{MockResponse, MockTransport};

    #[test]
    fn make_request_default() {
        let transport = MockTransport::new();

        let body = serde_json::builder::ObjectBuilder::new()
                       .insert("field_1", 42)
                       .insert("field_2", "hello")
                       .unwrap();

        let expected = ({
            let options = RequestOptions::new().with_accept_json().with_json_body(&body);
            transport.post(vec!["foo"], options).unwrap()
        },
                        ());

        let got = {
            let mut action = CreateDocument::new(&transport, "/foo", &body).unwrap();
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_document_id() {
        let transport = MockTransport::new();

        let body = serde_json::builder::ObjectBuilder::new()
                       .insert("_id", "bar")
                       .insert("field_1", 42)
                       .insert("field_2", "hello")
                       .unwrap();

        let expected_request = {
            let options = RequestOptions::new().with_accept_json().with_json_body(&body);
            transport.post(vec!["foo"], options).unwrap()
        };

        let (got_request, _) = {
            let mut action = CreateDocument::new(&transport, "/foo", &body)
                                 .unwrap()
                                 .with_document_id("bar");
            action.make_request().unwrap()
        };

        assert_eq!(expected_request, got_request);
    }

    #[test]
    fn take_response_created() {
        let rev = Revision::parse("1-967a00dff5e02add41819138abb3284d").unwrap();
        let response = MockResponse::new(StatusCode::Created).build_json_body(|x| {
            x.insert("ok", true)
             .insert("id", "foo")
             .insert("rev", rev.to_string())
        });
        let expected = (DocumentId::from("foo"), rev);
        let got = CreateDocument::<MockTransport, ()>::take_response(response, ()).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_conflict() {
        let error = "conflict";
        let reason = "Document update conflict.";
        let response = MockResponse::new(StatusCode::Conflict).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        });
        match CreateDocument::<MockTransport, ()>::take_response(response, ()) {
            Err(Error::DocumentConflict(ref error_response)) if error == error_response.error() &&
                                                                reason ==
                                                                error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn take_response_unauthorized() {
        let error = "unauthorized";
        let reason = "Authentication required.";
        let response = MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        });
        match CreateDocument::<MockTransport, ()>::take_response(response, ()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
