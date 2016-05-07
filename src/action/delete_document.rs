use {Error, IntoDocumentPath, Revision};
use document::WriteDocumentResponse;
use transport::{Action, RequestOptions, Response, StatusCode, Transport};
use transport::production::HyperTransport;

pub struct DeleteDocument<'a, T: Transport + 'a, P: IntoDocumentPath> {
    transport: &'a T,
    doc_path: P,
    revision: &'a Revision,
}

impl<'a, P: IntoDocumentPath, T: Transport + 'a> DeleteDocument<'a, T, P> {
    #[doc(hidden)]
    pub fn new(transport: &'a T, doc_path: P, revision: &'a Revision) -> Self
        where P: IntoDocumentPath
    {
        DeleteDocument {
            transport: transport,
            doc_path: doc_path,
            revision: revision,
        }
    }
}

impl<'a, P: IntoDocumentPath> DeleteDocument<'a, HyperTransport, P> {
    pub fn run(self) -> Result<Revision, Error> {
        self.transport.exec_sync(self)
    }
}

impl<'a, P: IntoDocumentPath, T: Transport + 'a> Action<T> for DeleteDocument<'a, T, P> {
    type Output = Revision;
    type State = ();

    fn make_request(self) -> Result<(T::Request, Self::State), Error> {
        let doc_path = try!(self.doc_path.into_document_path());
        let options = RequestOptions::new().with_accept_json().with_revision_query(self.revision);
        let request = try!(self.transport.delete(doc_path.iter(), options));
        Ok((request, ()))
    }

    fn take_response<R: Response>(response: R, _state: Self::State) -> Result<Self::Output, Error> {
        match response.status_code() {
            StatusCode::Ok => {
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

    use super::*;
    use {DocumentPath, Error, Revision};
    use transport::{Action, RequestOptions, StatusCode, Transport};
    use transport::testing::{MockResponse, MockTransport};

    #[test]
    fn make_request_default() {

        let transport = MockTransport::new();
        let rev1 = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();

        let expected = ({
            let options = RequestOptions::new().with_accept_json().with_revision_query(&rev1);
            transport.delete(vec!["foo", "bar"], options).unwrap()
        },
                        ());

        let got = {
            let action = DeleteDocument::new(&transport, "/foo/bar", &rev1);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_ok() {
        let rev = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        let response = MockResponse::new(StatusCode::Ok).build_json_body(|x| {
            x.insert("ok", "true")
             .insert("id", "bar")
             .insert("rev", rev.to_string())
        });
        let expected = rev;
        let got = DeleteDocument::<MockTransport, DocumentPath>::take_response(response, ())
                      .unwrap();
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
        match DeleteDocument::<MockTransport, DocumentPath>::take_response(response, ()) {
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
        match DeleteDocument::<MockTransport, DocumentPath>::take_response(response, ()) {
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
        match DeleteDocument::<MockTransport, DocumentPath>::take_response(response, ()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
