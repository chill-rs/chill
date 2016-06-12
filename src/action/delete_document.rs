use {Error, IntoDocumentPath, Revision, std};
use action::query_keys::*;
use document::WriteDocumentResponse;
use transport::{JsonResponse, JsonResponseDecoder, Request, StatusCode, Transport};

pub struct DeleteDocument<'a, T: Transport + 'a, P: IntoDocumentPath> {
    transport: &'a T,
    doc_path: Option<P>,
    revision: &'a Revision,
}

impl<'a, P: IntoDocumentPath, T: Transport + 'a> DeleteDocument<'a, T, P> {
    #[doc(hidden)]
    pub fn new(transport: &'a T, doc_path: P, revision: &'a Revision) -> Self
        where P: IntoDocumentPath
    {
        DeleteDocument {
            transport: transport,
            doc_path: Some(doc_path),
            revision: revision,
        }
    }

    pub fn run(mut self) -> Result<Revision, Error> {
        self.transport.send(try!(self.make_request()),
                            JsonResponseDecoder::new(handle_response))
    }

    fn make_request(&mut self) -> Result<Request, Error> {
        let doc_path = try!(std::mem::replace(&mut self.doc_path, None).unwrap().into_document_path());
        Ok(self.transport.delete(doc_path.iter()).with_accept_json().with_query(RevisionQueryKey, self.revision))
    }
}

fn handle_response(response: JsonResponse) -> Result<Revision, Error> {
    match response.status_code() {
        StatusCode::Ok => {
            let body: WriteDocumentResponse = try!(response.decode_content());
            Ok(body.revision)
        }
        StatusCode::Conflict => Err(Error::document_conflict(&response)),
        StatusCode::NotFound => Err(Error::not_found(&response)),
        StatusCode::Unauthorized => Err(Error::unauthorized(&response)),
        _ => Err(Error::server_response(&response)),
    }
}

#[cfg(test)]
mod tests {

    use {Error, Revision};
    use super::*;
    use transport::{JsonResponseBuilder, MockTransport, StatusCode, Transport};

    #[test]
    fn make_request_default() {

        let transport = MockTransport::new();
        let expected = transport.delete(vec!["foo", "bar"])
            .with_accept_json()
            .with_query_literal("rev", "1-1234567890abcdef1234567890abcdef");

        let got = {
            let rev = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
            let mut action = DeleteDocument::new(&transport, "/foo/bar", &rev);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn handle_response_ok() {

        let response = JsonResponseBuilder::new(StatusCode::Ok)
            .with_json_content_raw(r#"{"ok":true,"id":"bar","rev":"42-1234567890abcdef1234567890abcdef"}"#)
            .unwrap();

        let expected = Revision::parse("42-1234567890abcdef1234567890abcdef").unwrap();
        let got = super::handle_response(response).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn handle_response_conflict() {

        let response = JsonResponseBuilder::new(StatusCode::Conflict)
            .with_json_content_raw(r#"{"error":"conflict","reason":"Document update conflict."}"#)
            .unwrap();

        match super::handle_response(response) {
            Err(Error::DocumentConflict(ref error_response)) if error_response.error() == "conflict" &&
                                                                error_response.reason() ==
                                                                "Document update conflict." => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn handle_response_not_found() {

        let response = JsonResponseBuilder::new(StatusCode::NotFound)
            .with_json_content_raw(r#"{"error":"not_found","reason":"no_db_file"}"#)
            .unwrap();

        match super::handle_response(response) {
            Err(Error::NotFound(ref error_response)) if error_response.error() == "not_found" &&
                                                        error_response.reason() == "no_db_file" => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn handle_response_unauthorized() {

        let response = JsonResponseBuilder::new(StatusCode::Unauthorized)
            .with_json_content_raw(r#"{"error":"unauthorized","reason":"Authentication required."}"#)
            .unwrap();

        match super::handle_response(response) {
            Err(Error::Unauthorized(ref error_response)) if error_response.error() == "unauthorized" &&
                                                            error_response.reason() == "Authentication required." => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
