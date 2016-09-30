//! Defines an action for reading a document from the CouchDB server.

use {DatabaseName, Document, Error, IntoDocumentPath, Revision, std};
use action::query_keys::*;
use document::JsonDecodableDocument;
use transport::{JsonResponse, JsonResponseDecoder, Request, StatusCode, Transport};

/// Reads a document from the CouchDB server and returns the result.
///
/// Chill reads the document by sending an HTTP request to `GET` from the
/// document's path. For more details about documents and how to read them,
/// please see the CouchDB documentation.
///
/// # Errors
///
/// The following are _some_ errors that may occur when reading a document.
///
/// <table>
/// <tr>
///  <td><code>Error::NotFound</code></td>
///  <td>The database or document does not exist.</td>
/// </tr>
/// <tr>
///  <td><code>Error::Unauthorized</code></td>
///  <td>The client lacks permission to read the document.</td>
/// </tr>
/// </table>
///
/// # Examples
///
/// The following program demonstrates reading a document.
///
/// ```
/// extern crate chill;
/// extern crate serde_json;
///
/// let server = chill::testing::FakeServer::new().unwrap();
/// let client = chill::Client::new(server.uri()).unwrap();
///
/// client.create_database("/baseball").run().unwrap();
///
/// let content = serde_json::builder::ObjectBuilder::new()
///                   .insert("name", "Babe Ruth")
///                   .insert("nickname", "The Bambino")
///                   .build();
///
/// let (doc_id, rev) = client.create_document("/baseball", &content)
///                            .run()
///                            .unwrap();
///
/// let doc = client.read_document(("/baseball", doc_id))
///                 .run()
///                 .unwrap();
///
/// assert_eq!(1, rev.sequence_number());
/// assert_eq!(content, doc.get_content::<serde_json::Value>().unwrap());
/// ```
///
pub struct ReadDocument<'a, T: Transport + 'a, P: IntoDocumentPath> {
    transport: &'a T,
    doc_path: Option<P>,
    revision: Option<&'a Revision>,
    attachment_content: Option<AttachmentContent>,
}

impl<'a, T: Transport + 'a, P: IntoDocumentPath> ReadDocument<'a, T, P> {
    #[doc(hidden)]
    pub fn new(transport: &'a T, doc_path: P) -> Self {
        ReadDocument {
            transport: transport,
            doc_path: Some(doc_path),
            revision: None,
            attachment_content: None,
        }
    }

    /// Modifies the action to read the document of the given revision.
    ///
    /// The `with_revision` method abstracts the `rev` query parameter of the
    /// HTTP request `GET /db/docid`. By default, the CouchDB
    ///
    pub fn with_revision(mut self, revision: &'a Revision) -> Self {
        self.revision = Some(revision);
        self
    }

    /// Modifies the action to retrieve (or not retrieve) attachment content
    /// with the document.
    ///
    /// By default, the CouchDB server sends stubs containing no content for all
    /// attachments.
    ///
    pub fn with_attachment_content(mut self, attachment_content: AttachmentContent) -> Self {
        self.attachment_content = Some(attachment_content);
        self
    }

    /// Executes the action and waits for the result.
    pub fn run(mut self) -> Result<Document, Error> {
        let (request, db_name) = try!(self.make_request());
        self.transport.send(request,
                            JsonResponseDecoder::new(move |response| handle_response(response, db_name)))
    }

    fn make_request(&mut self) -> Result<(Request, DatabaseName), Error> {
        let doc_path = try!(std::mem::replace(&mut self.doc_path, None).unwrap().into_document_path());
        let db_name = doc_path.database_name().clone();
        let request = self.transport.get(doc_path.iter()).with_accept_json();

        let request = match self.attachment_content {
            None => request,
            Some(AttachmentContent::None) => request.with_query(AttachmentsQueryKey, &false),
            Some(AttachmentContent::All) => request.with_query(AttachmentsQueryKey, &true),
        };

        let request = match self.revision {
            None => request,
            Some(rev) => request.with_query(RevisionQueryKey, rev),
        };

        Ok((request, db_name))
    }
}

fn handle_response(response: JsonResponse, db_name: DatabaseName) -> Result<Document, Error> {
    match response.status_code() {
        StatusCode::Ok => {
            let decoded_doc: JsonDecodableDocument = try!(response.decode_content());
            Ok(Document::new_from_decoded(db_name, decoded_doc))
        }
        StatusCode::NotFound => Err(Error::not_found(&response)),
        StatusCode::Unauthorized => Err(Error::unauthorized(&response)),
        _ => Err(Error::server_response(&response)),
    }
}

/// Specifies the attachments, if any, for which the CouchDB server should send
/// content.
///
/// `AttachmentContent` abstracts the `attachments` query parameter of the HTTP
/// request `GET /db/doc_id`. Chill does not yet support the `atts_since` query
/// parameter—see [issue #37](https://github.com/chill-rs/chill/issues/37).
///
#[derive(Debug)]
pub enum AttachmentContent {
    /// Specifies to send no content for all attachments.
    None,

    /// Specifies to send content for all attachments.
    All,
}

#[cfg(test)]
mod tests {

    use {DatabaseName, Error, Revision};
    use super::*;
    use document::DocumentBuilder;
    use transport::{JsonResponseBuilder, MockTransport, StatusCode, Transport};

    #[test]
    fn make_request_default() {

        let transport = MockTransport::new();
        let expected = (transport.get(vec!["foo", "bar"]).with_accept_json(), DatabaseName::from("foo"));

        let got = {
            let mut action = ReadDocument::new(&transport, "/foo/bar");
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_revision() {

        let transport = MockTransport::new();

        let expected = (transport.get(vec!["foo", "bar"])
            .with_accept_json()
            .with_query_literal("rev", "1-1234567890abcdef1234567890abcdef"),
                        DatabaseName::from("foo"));

        let rev = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        let got = {
            let mut action = ReadDocument::new(&transport, "/foo/bar").with_revision(&rev);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_attachment_content_none() {

        let transport = MockTransport::new();

        let expected = (transport.get(vec!["foo", "bar"]).with_accept_json().with_query_literal("attachments", "false"),
                        DatabaseName::from("foo"));

        let got = {
            let mut action = ReadDocument::new(&transport, "/foo/bar").with_attachment_content(AttachmentContent::None);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_attachment_content_all() {

        let transport = MockTransport::new();

        let expected = (transport.get(vec!["foo", "bar"]).with_accept_json().with_query_literal("attachments", "true"),
                        DatabaseName::from("foo"));

        let got = {
            let mut action = ReadDocument::new(&transport, "/foo/bar").with_attachment_content(AttachmentContent::All);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn handle_response_ok() {

        let response = JsonResponseBuilder::new(StatusCode::Ok)
            .with_json_content_raw(r#"{"_id": "bar", "_rev": "1-1234567890abcdef1234567890abcdef", "field": 42}"#)
            .unwrap();

        let rev = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();

        let expected = DocumentBuilder::new("/foo/bar", rev)
            .build_content(|x| x.insert("field", 42))
            .unwrap();

        let got = super::handle_response(response, DatabaseName::from("foo")).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn handle_response_not_found() {

        let response = JsonResponseBuilder::new(StatusCode::NotFound)
            .with_json_content_raw(r#"{"error":"not_found","reason":"no_db_file"}"#)
            .unwrap();

        match super::handle_response(response, DatabaseName::from("foo")) {
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

        match super::handle_response(response, DatabaseName::from("foo")) {
            Err(Error::Unauthorized(ref error_response)) if error_response.error() == "unauthorized" &&
                                                            error_response.reason() == "Authentication required." => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
