use DatabaseName;
use Document;
use DocumentPathRef;
use document::JsonDecodableDocument;
use Error;
use IntoDocumentPath;
use transport::{Action, HyperTransport, RequestOptions, Response, StatusCode, Transport};

pub struct ReadDocument<'a, T: Transport + 'a> {
    transport: &'a T,
    doc_path: DocumentPathRef<'a>,
}

impl<'a, T: Transport + 'a> ReadDocument<'a, T> {
    #[doc(hidden)]
    pub fn new<P: IntoDocumentPath<'a>>(transport: &'a T, doc_path: P) -> Result<Self, Error> {
        Ok(ReadDocument {
            transport: transport,
            doc_path: try!(doc_path.into_document_path()),
        })
    }
}

impl<'a> ReadDocument<'a, HyperTransport> {
    pub fn run(self) -> Result<<Self as Action<HyperTransport>>::Output, Error> {
        self.transport.exec_sync(self)
    }
}

impl<'a, T: Transport + 'a> Action<T> for ReadDocument<'a, T> {
    type Output = Document;
    type State = DatabaseName;

    fn make_request(&mut self) -> Result<(T::Request, Self::State), Error> {
        let db_name = DatabaseName::from(self.doc_path.database_name());
        let options = RequestOptions::new().with_accept_json();
        let request = try!(self.transport.get(self.doc_path, options));
        Ok((request, db_name))
    }

    fn take_response<R: Response>(response: R,
                                  db_name: Self::State)
                                  -> Result<Self::Output, Error> {
        match response.status_code() {
            StatusCode::Ok => {
                let decoded_doc: JsonDecodableDocument = try!(response.decode_json_body());
                Ok(Document::new_from_decoded(db_name, decoded_doc))
            }
            StatusCode::NotFound => Err(Error::not_found(response)),
            StatusCode::Unauthorized => Err(Error::unauthorized(response)),
            _ => Err(Error::server_response(response)),
        }
    }
}

#[cfg(test)]
mod tests {

    use DatabaseName;
    use DatabaseNameRef;
    use DocumentIdRef;
    use document::DocumentBuilder;
    use Error;
    use Revision;
    use super::*;
    use transport::{Action, MockResponse, MockTransport, RequestOptions, StatusCode, Transport};

    #[test]
    fn make_request_default() {
        let transport = MockTransport::new();

        let expected = ({
            let options = RequestOptions::new().with_accept_json();
            transport.get(vec!["foo", "bar"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let mut action = ReadDocument::new(&transport, "/foo/bar").unwrap();
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_ok() {
        let db_name = DatabaseNameRef::from("foo");
        let doc_id = DocumentIdRef::from("bar");
        let rev = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        let response = MockResponse::new(StatusCode::Ok).build_json_body(|x| {
            x.insert("_id", doc_id.to_string())
             .insert("_rev", rev.to_string())
             .insert("field_1", 42)
             .insert("field_2", "hello")
        });

        let expected = DocumentBuilder::new((db_name, doc_id), rev)
                           .build_content(|x| {
                               x.insert("field_1", 42)
                                .insert("field_2", "hello")
                           })
                           .unwrap();

        let got = ReadDocument::<MockTransport>::take_response(response,
                                                               DatabaseName::from(db_name))
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_not_found() {
        let error = "not_found";
        let reason = "no_db_file";
        let response = MockResponse::new(StatusCode::NotFound).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        });
        match ReadDocument::<MockTransport>::take_response(response, DatabaseName::from("foo")) {
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
        match ReadDocument::<MockTransport>::take_response(response, DatabaseName::from("foo")) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
