use {Error, IntoDatabasePath, std};
use transport::{JsonResponse, JsonResponseDecoder, Request, StatusCode, Transport};

pub struct CreateDatabase<'a, T: Transport + 'a, P: IntoDatabasePath> {
    transport: &'a T,
    db_path: Option<P>,
}

impl<'a, P: IntoDatabasePath, T: Transport + 'a> CreateDatabase<'a, T, P> {
    #[doc(hidden)]
    pub fn new(transport: &'a T, db_path: P) -> Self {
        CreateDatabase {
            transport: transport,
            db_path: Some(db_path),
        }
    }

    pub fn run(mut self) -> Result<(), Error> {
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
        Ok(self.transport.put(db_path.iter()).with_accept_json())
    }
}

fn handle_response(response: JsonResponse) -> Result<(), Error> {
    match response.status_code() {
        StatusCode::Created => Ok(()),
        StatusCode::PreconditionFailed => Err(Error::database_exists(&response)),
        StatusCode::Unauthorized => Err(Error::unauthorized(&response)),
        _ => Err(Error::server_response(&response)),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use Error;
    use transport::{JsonResponseBuilder, MockTransport, StatusCode, Transport};

    #[test]
    fn make_request_default() {
        let transport = MockTransport::new();
        let expected = transport.put(vec!["foo"]).with_accept_json();

        let got = {
            let mut action = CreateDatabase::new(&transport, "/foo");
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn handle_response_created() {
        let response = JsonResponseBuilder::new(StatusCode::Created)
            .with_json_content_raw(r#"{"ok":true}"#)
            .unwrap();
        super::handle_response(response).unwrap();
    }

    #[test]
    fn handle_response_precondition_failed() {
        let response = JsonResponseBuilder::new(StatusCode::PreconditionFailed)
            .with_json_content_raw(
                r#"{"error":"file_exists","reason":"The database could not be created, the file already exists."}"#,
            )
            .unwrap();
        match super::handle_response(response) {
            Err(Error::DatabaseExists(ref error_response))
                if error_response.error() == "file_exists" &&
                       error_response.reason() ==
                           "The database could not be created, the file \
                                                               already exists." => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn handle_response_unauthorized() {
        let response = JsonResponseBuilder::new(StatusCode::Unauthorized)
            .with_json_content_raw(
                r#"{"error": "unauthorized", "reason": "Authentication required."}"#,
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
