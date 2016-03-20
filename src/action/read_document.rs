use DatabaseName;
use Document;
use document::JsonDecodableDocument;
use Error;
use IntoDocumentPath;
use transport::{RequestOptions, Response, StatusCode, Transport};

pub struct ReadDocument<'a, P, T>
    where P: IntoDocumentPath<'a>,
          T: Transport + 'a
{
    transport: &'a T,
    doc_path: P,
}

impl<'a, P, T> ReadDocument<'a, P, T>
    where P: IntoDocumentPath<'a>,
          T: Transport + 'a
{
    #[doc(hidden)]
    pub fn new(transport: &'a T, doc_path: P) -> Self {
        ReadDocument {
            transport: transport,
            doc_path: doc_path,
        }
    }

    pub fn run(self) -> Result<Document, Error> {

        let doc_path = try!(self.doc_path.into_document_path());

        let response = try!(self.transport
                                .get(doc_path, RequestOptions::new().with_accept_json()));

        match response.status_code() {
            StatusCode::Ok => {
                let decoded_doc: JsonDecodableDocument = try!(response.decode_json_body());
                Ok(Document::new_from_decoded(DatabaseName::from(doc_path.database_name()),
                                              decoded_doc))
            }
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
    fn read_document_ok_basic() {

        let transport = MockTransport::new();
        transport.push_response(MockResponse::new(StatusCode::Ok).build_json_body(|x| {
            x.insert("_id", "document_id")
             .insert("_rev", "1-967a00dff5e02add41819138abb3284d")
             .insert("field_1", 42)
             .insert("field_2", "hello")
        }));

        let expected = DocumentBuilder::new("/database_name/document_id",
                                            Revision::parse("1-967a00dff5e02add41819138abb3284d")
                                                .unwrap())
                           .build_content(|x| {
                               x.insert("field_1", 42)
                                .insert("field_2", "hello")
                           })
                           .unwrap();

        let doc = ReadDocument::new(&transport, "/database_name/document_id").run().unwrap();
        assert_eq!(expected, doc);

        let expected = {
            MockRequestMatcher::new()
                .get(&["database_name", "document_id"], |x| x.with_accept_json())
        };
        assert_eq!(expected, transport.extract_requests());
    }

    #[test]
    fn read_document_nok_not_found() {

        let transport = MockTransport::new();
        let error = "not_found";
        let reason = "missing";
        transport.push_response(MockResponse::new(StatusCode::NotFound).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        match ReadDocument::new(&transport, "/database_name/document_id").run() {
            Err(Error::NotFound(ref error_response)) if error == error_response.error() &&
                                                        reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn read_document_nok_unauthorized() {

        let transport = MockTransport::new();
        let error = "unauthorized";
        let reason = "Authentication required.";
        transport.push_response(MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        match ReadDocument::new(&transport, "/database_name/document_id").run() {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
