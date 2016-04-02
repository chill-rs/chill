use prelude_impl::*;

pub struct DeleteDocument<'a, T: Transport + 'a> {
    transport: &'a T,
    doc_path: DocumentPathRef<'a>,
    revision: &'a Revision,
}

impl<'a, T: Transport + 'a> DeleteDocument<'a, T> {
    #[doc(hidden)]
    pub fn new<P>(transport: &'a T, doc_path: P, revision: &'a Revision) -> Result<Self, Error>
        where P: IntoDocumentPath<'a>
    {
        Ok(DeleteDocument {
            transport: transport,
            doc_path: try!(doc_path.into_document_path()),
            revision: revision,
        })
    }
}

impl<'a> DeleteDocument<'a, HyperTransport> {
    pub fn run(self) -> Result<Revision, Error> {
        self.transport.exec_sync(self)
    }
}

impl<'a, T: Transport + 'a> Action<T> for DeleteDocument<'a, T> {
    type Output = Revision;
    type State = ();

    fn make_request(&mut self) -> Result<(T::Request, Self::State), Error> {
        let options = RequestOptions::new().with_accept_json().with_revision_query(self.revision);
        let request = try!(self.transport.delete(self.doc_path, options));
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

    use prelude_impl::*;
    use super::DeleteDocument;

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
            let mut action = DeleteDocument::new(&transport, "/foo/bar", &rev1).unwrap();
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
        let got = DeleteDocument::<MockTransport>::take_response(response, ()).unwrap();
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
        match DeleteDocument::<MockTransport>::take_response(response, ()) {
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
        match DeleteDocument::<MockTransport>::take_response(response, ()) {
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
        match DeleteDocument::<MockTransport>::take_response(response, ()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
