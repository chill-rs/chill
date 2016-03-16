use Document;
use document::WriteDocumentResponse;
use Error;
use Revision;
use transport::{RequestOptions, Response, StatusCode, Transport};

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

    pub fn run(self) -> Result<Revision, Error> {

        let response = try!(self.transport
                                .put(self.doc.path().iter(),
                                     RequestOptions::new()
                                         .with_accept_json()
                                         .with_revision_query(&self.doc.revision())
                                         .with_json_body(self.doc)));

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
    use super::*;
    use transport::{MockRequestMatcher, MockResponse, MockTransport, StatusCode};

    #[test]
    fn update_document_ok_basic() {

        let transport = MockTransport::new();

        let original_revision: Revision = "1-1234567890abcdef1234567890abcdef".parse().unwrap();

        let doc = DocumentBuilder::new("/database_name/document_id", original_revision.clone())
                      .build_content(|x| {
                          x.insert("field_1", 42)
                           .insert("field_2", "hello")
                      })
                      .unwrap();

        let new_revision: Revision = "2-fedcba0987654321fedcba0987654321".parse().unwrap();

        transport.push_response(MockResponse::new(StatusCode::Created).build_json_body(|x| {
            x.insert("ok", true)
             .insert("id", "document_id")
             .insert("rev", new_revision.to_string())
        }));

        let got = UpdateDocument::new(&transport, &doc).run().unwrap();
        assert_eq!(new_revision, got);

        let expected = {
            MockRequestMatcher::new().put(&["database_name", "document_id"], |x| {
                x.with_accept_json()
                 .with_revision_query(&original_revision)
                 .build_json_body(|x| {
                     x.insert("field_1", 42)
                      .insert("field_2", "hello")
                 })
            })
        };

        assert_eq!(expected, transport.extract_requests());
    }

    #[test]
    fn update_document_nok_document_conflict() {

        let transport = MockTransport::new();
        let error = "conflict";
        let reason = "Document update conflict.";
        transport.push_response(MockResponse::new(StatusCode::Conflict).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        let doc = DocumentBuilder::new("/database_name/document_id",
                                       Revision::parse("42-1234567890abcdef1234567890abcdef")
                                           .unwrap())
                      .unwrap();

        match UpdateDocument::new(&transport, &doc).run() {
            Err(Error::DocumentConflict(ref error_response)) if error == error_response.error() &&
                                                                reason ==
                                                                error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn update_document_nok_not_found() {

        let transport = MockTransport::new();
        let error = "not_found";
        let reason = "no_db_file";
        transport.push_response(MockResponse::new(StatusCode::NotFound).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        let doc = DocumentBuilder::new("/database_name/document_id",
                                       Revision::parse("42-1234567890abcdef1234567890abcdef")
                                           .unwrap())
                      .unwrap();

        match UpdateDocument::new(&transport, &doc).run() {
            Err(Error::NotFound(ref error_response)) if error == error_response.error() &&
                                                        reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn update_document_nok_unauthorized() {

        let transport = MockTransport::new();
        let error = "unauthorized";
        let reason = "Authentication required.";
        transport.push_response(MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        let doc = DocumentBuilder::new("/database_name/document_id",
                                       Revision::parse("42-1234567890abcdef1234567890abcdef")
                                           .unwrap())
                      .unwrap();

        match UpdateDocument::new(&transport, &doc).run() {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

}
