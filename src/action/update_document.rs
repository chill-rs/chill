use {Document, Error, Revision};
use document::WriteDocumentResponse;
use transport::{Action, RequestOptions, Response, StatusCode, Transport};
use transport::production::HyperTransport;

pub struct UpdateDocument<'a, T>
    where T: Transport + 'a
{
    transport: &'a T,
    doc: &'a Document,
}

impl<'a, T> UpdateDocument<'a, T>
    where T: Transport + 'a
{
    #[doc(hidden)]
    pub fn new(transport: &'a T, doc: &'a Document) -> Self {
        UpdateDocument {
            transport: transport,
            doc: doc,
        }
    }
}

impl<'a> UpdateDocument<'a, HyperTransport> {
    pub fn run(self) -> Result<Revision, Error> {
        self.transport.exec_sync(self)
    }
}

impl<'a, T: Transport + 'a> Action<T> for UpdateDocument<'a, T> {
    type Output = Revision;
    type State = ();

    fn make_request(self) -> Result<(T::Request, Self::State), Error> {
        let options = RequestOptions::new()
                          .with_accept_json()
                          .with_revision_query(&self.doc.revision())
                          .with_json_body(self.doc);
        let request = try!(self.transport.put(self.doc.path().iter(), options));
        Ok((request, ()))
    }

    fn take_response<R: Response>(response: R, _state: Self::State) -> Result<Self::Output, Error> {
        match response.status_code() {
            StatusCode::Created => {
                let body: WriteDocumentResponse = try!(response.decode_json_body());
                Ok(body.revision)
            }
            StatusCode::Conflict => Err(Error::document_conflict(response)),
            StatusCode::NotFound => Err(Error::not_found(response)),
            StatusCode::Unauthorized => Err(Error::unauthorized(response)),
            _ => Err(Error::server_response(response)),
        }
    }
}

#[cfg(test)]
mod tests {

    use document::DocumentBuilder;
    use Error;
    use Revision;
    use serde_json;
    use super::UpdateDocument;
    use transport::{Action, RequestOptions, StatusCode, Transport};
    use transport::testing::{MockResponse, MockTransport};

    #[test]
    fn make_request_default() {
        let transport = MockTransport::new();

        let rev1 = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        let doc = DocumentBuilder::new("/foo/bar", rev1.clone())
                      .build_content(|x| {
                          x.insert("field_1", 42)
                           .insert("field_2", "hello")
                      })
                      .unwrap();

        let expected = ({
            let body = serde_json::builder::ObjectBuilder::new()
                           .insert("field_1", 42)
                           .insert("field_2", "hello")
                           .unwrap();
            let options = RequestOptions::new()
                              .with_accept_json()
                              .with_revision_query(&rev1)
                              .with_json_body(&body);
            transport.put(vec!["foo", "bar"], options).unwrap()
        },
                        ());

        let got = {
            let action = UpdateDocument::new(&transport, &doc);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_created() {
        let rev = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        let response = MockResponse::new(StatusCode::Created).build_json_body(|x| {
            x.insert("ok", true)
             .insert("id", "bar")
             .insert("rev", rev.to_string())
        });
        let expected = rev;
        let got = UpdateDocument::<MockTransport>::take_response(response, ()).unwrap();
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
        match UpdateDocument::<MockTransport>::take_response(response, ()) {
            Err(Error::DocumentConflict(ref error_response)) if error == error_response.error() &&
                                                                reason ==
                                                                error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn take_response_not_found() {
        let error = "not_found";
        let reason = "no_db_file";
        let response = MockResponse::new(StatusCode::NotFound).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        });
        match UpdateDocument::<MockTransport>::take_response(response, ()) {
            Err(Error::NotFound(ref error_response)) if error == error_response.error() &&
                                                        reason == error_response.reason() => (),
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
        match UpdateDocument::<MockTransport>::take_response(response, ()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
