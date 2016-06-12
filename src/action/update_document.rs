use {Document, Error, Revision};
use action::query_keys::*;
use document::WriteDocumentResponse;
use transport::{JsonResponse, JsonResponseDecoder, Request, StatusCode, Transport};

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

    pub fn run(mut self) -> Result<Revision, Error> {
        self.transport.send(try!(self.make_request()),
                            JsonResponseDecoder::new(handle_response))
    }

    fn make_request(&mut self) -> Result<Request, Error> {
        self.transport
            .put(self.doc.path().iter())
            .with_accept_json()
            .with_query(RevisionQueryKey, self.doc.revision())
            .with_json_content(&self.doc)
    }
}

fn handle_response(response: JsonResponse) -> Result<Revision, Error> {
    match response.status_code() {
        StatusCode::Created => {
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

    use {Error, Revision, serde_json};
    use super::*;
    use document::DocumentBuilder;
    use transport::{JsonResponseBuilder, MockTransport, StatusCode, Transport};

    #[test]
    fn make_request_default() {

        let transport = MockTransport::new();

        let doc = DocumentBuilder::new("/foo/bar",
                                       Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap())
            .build_content(|x| {
                x.insert("field_1", 42)
                    .insert("field_2", "hello")
            })
            .unwrap();

        let request_content = serde_json::builder::ObjectBuilder::new()
            .insert("field_1", 42)
            .insert("field_2", "hello")
            .unwrap();

        let expected = transport.put(vec!["foo", "bar"])
            .with_accept_json()
            .with_query_literal("rev", "1-1234567890abcdef1234567890abcdef")
            .with_json_content(&request_content)
            .unwrap();

        let got = {
            let mut action = UpdateDocument::new(&transport, &doc);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn handle_response_created() {

        let response = JsonResponseBuilder::new(StatusCode::Created)
            .with_json_content_raw(r#"{"ok":true,"id":"bar","rev":"1-1234567890abcdef1234567890abcdef"}"#)
            .unwrap();

        let expected = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
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
