use DocumentPath;
use document::WriteDocumentResponse;
use Error;
use Revision;
use transport::{RequestOptions, Response, StatusCode, Transport};

pub struct DeleteDocument<'a, P, T>
    where P: DocumentPath,
          T: Transport + 'a
{
    transport: &'a T,
    doc_path: P,
    revision: &'a Revision,
}

impl<'a, P, T> DeleteDocument<'a, P, T>
    where P: DocumentPath,
          T: Transport + 'a
{
    #[doc(hidden)]
    pub fn new(transport: &'a T, doc_path: P, revision: &'a Revision) -> Self {
        DeleteDocument {
            transport: transport,
            doc_path: doc_path,
            revision: revision,
        }
    }

    pub fn run(self) -> Result<Revision, Error> {

        let (db_name, doc_id) = try!(self.doc_path.document_path());

        let response = try!(self.transport.delete(&[db_name.as_ref(), doc_id.as_ref()],
                                                  RequestOptions::new()
                                                      .with_accept_json()
                                                      .with_revision_query(self.revision)));

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

    use Error;
    use Revision;
    use super::*;
    use transport::{MockRequestMatcher, MockResponse, MockTransport, StatusCode};

    #[test]
    fn delete_document_ok_basic() {

        let transport = MockTransport::new();

        let original_revision: Revision = "1-1234567890abcdef1234567890abcdef".parse().unwrap();
        let new_revision: Revision = "2-fedcba0987654321fedcba0987654321".parse().unwrap();

        transport.push_response(MockResponse::new(StatusCode::Ok).build_json_body(|x| {
            x.insert("ok", true)
             .insert("id", "document_id")
             .insert("rev", new_revision.to_string())
        }));

        let got = DeleteDocument::new(&transport, "/database_name/document_id", &original_revision)
                      .run()
                      .unwrap();
        assert_eq!(new_revision, got);

        let expected = {
            MockRequestMatcher::new().delete(&["database_name", "document_id"], |x| {
                x.with_accept_json()
                 .with_revision_query(&original_revision)
            })
        };

        assert_eq!(expected, transport.extract_requests());
    }

    #[test]
    fn delete_document_nok_document_conflict() {

        let transport = MockTransport::new();
        let error = "conflict";
        let reason = "Document update conflict.";
        transport.push_response(MockResponse::new(StatusCode::Conflict).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        let revision = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        match DeleteDocument::new(&transport, "/database_name/document_id", &revision).run() {
            Err(Error::DocumentConflict(ref error_response)) if error == error_response.error() &&
                                                                reason ==
                                                                error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn delete_document_nok_not_found() {

        let transport = MockTransport::new();
        let error = "not_found";
        let reason = "no_db_file";
        transport.push_response(MockResponse::new(StatusCode::Conflict).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        let revision = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        match DeleteDocument::new(&transport, "/database_name/document_id", &revision).run() {
            Err(Error::DocumentConflict(ref error_response)) if error == error_response.error() &&
                                                                reason ==
                                                                error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn delete_document_nok_unauthorized() {

        let transport = MockTransport::new();
        let error = "unauthorized";
        let reason = "Authentication required.";
        transport.push_response(MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        let revision = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        match DeleteDocument::new(&transport, "/database_name/document_id", &revision).run() {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
