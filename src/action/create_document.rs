use {DocumentId, DocumentPath, Error, IntoDatabasePath, Revision, serde, std};
use document::WriteDocumentResponse;
use transport::{JsonResponse, JsonResponseDecoder, Request, StatusCode, Transport};

pub struct CreateDocument<'a, T, P, C>
where
    C: serde::Serialize + 'a,
    P: IntoDatabasePath,
    T: Transport + 'a,
{
    transport: &'a T,
    db_path: Option<P>,
    content: &'a C,
    doc_id: Option<DocumentId>,
}

impl<'a, C, P, T> CreateDocument<'a, T, P, C>
where
    C: serde::Serialize + 'a,
    P: IntoDatabasePath,
    T: Transport + 'a,
{
    #[doc(hidden)]
    pub fn new(transport: &'a T, db_path: P, content: &'a C) -> Self {
        CreateDocument {
            transport: transport,
            db_path: Some(db_path),
            content: content,
            doc_id: None,
        }
    }

    pub fn with_document_id<D>(mut self, doc_id: D) -> Self
    where
        D: Into<DocumentId>,
    {
        self.doc_id = Some(doc_id.into());
        self
    }

    pub fn run(mut self) -> Result<(DocumentId, Revision), Error> {
        self.transport.send(
            try!(self.make_request()),
            JsonResponseDecoder::new(handle_response),
        )
    }

    fn make_request(&mut self) -> Result<Request, Error> {
        let db_path = try!(
            std::mem::replace(&mut self.db_path, None)
                .unwrap()
                .into_database_path()
        );

        let request = try!(
            match self.doc_id {
                None => self.transport.post(db_path.iter()),
                Some(ref doc_id) => {
                    let doc_path = DocumentPath::from((db_path, doc_id.clone()));
                    self.transport.put(doc_path.iter())
                }
            }.with_accept_json()
                .with_json_content(self.content)
        );

        Ok(request)
    }
}

fn handle_response(response: JsonResponse) -> Result<(DocumentId, Revision), Error> {
    match response.status_code() {
        StatusCode::Created => {
            let content: WriteDocumentResponse = try!(response.decode_content());
            Ok((content.doc_id, content.revision))
        }

        StatusCode::Conflict => Err(Error::document_conflict(&response)),
        StatusCode::Unauthorized => Err(Error::unauthorized(&response)),
        _ => Err(Error::server_response(&response)),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use {DocumentId, Error, Revision, serde_json};
    use transport::{JsonResponseBuilder, MockTransport, StatusCode, Transport};

    #[test]
    fn make_request_default() {

        let doc_content = serde_json::builder::ObjectBuilder::new()
            .insert("field", 42)
            .build();

        let transport = MockTransport::new();
        let expected = transport
            .post(vec!["foo"])
            .with_accept_json()
            .with_json_content(&doc_content)
            .unwrap();

        let got = {
            let mut action = CreateDocument::new(&transport, "/foo", &doc_content);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_document_id() {

        let doc_content = serde_json::builder::ObjectBuilder::new()
            .insert("field", 42)
            .build();

        let transport = MockTransport::new();

        let expected = transport
            .put(vec!["foo", "bar"])
            .with_accept_json()
            .with_json_content(&doc_content)
            .unwrap();

        let got = {
            let mut action = CreateDocument::new(&transport, "/foo", &doc_content).with_document_id("bar");
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn handle_response_created() {

        let response = JsonResponseBuilder::new(StatusCode::Created)
            .with_json_content_raw(
                r#"{"ok":true, "id": "foo", "rev": "1-1234567890abcdef1234567890abcdef"}"#,
            )
            .unwrap();

        let expected = (
            DocumentId::from("foo"),
            Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap(),
        );
        let got = super::handle_response(response).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_conflict() {

        let response = JsonResponseBuilder::new(StatusCode::Conflict)
            .with_json_content_raw(
                r#"{"error":"conflict","reason":"Document update conflict."}"#,
            )
            .unwrap();

        match super::handle_response(response) {
            Err(Error::DocumentConflict(ref error_response))
                if error_response.error() == "conflict" && error_response.reason() == "Document update conflict." => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn take_response_unauthorized() {

        let response = JsonResponseBuilder::new(StatusCode::Unauthorized)
            .with_json_content_raw(
                r#"{"error":"unauthorized","reason":"Authentication required."}"#,
            )
            .unwrap();

        match super::handle_response(response) {
            Err(Error::Unauthorized(ref error_response))
                if error_response.error() == "unauthorized" && error_response.reason() == "Authentication required." =>
                (),
            x @ _ => unexpected_result!(x),
        }
    }
}
