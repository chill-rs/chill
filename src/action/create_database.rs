use DatabasePath;
use Error;
use transport::{RequestOptions, Response, StatusCode, Transport};

pub struct CreateDatabase<'a, P, T>
    where P: DatabasePath,
          T: Transport + 'a
{
    transport: &'a T,
    db_path: P,
}

impl<'a, P, T> CreateDatabase<'a, P, T>
    where P: DatabasePath,
          T: Transport + 'a
{
    #[doc(hidden)]
    pub fn new(transport: &'a T, db_path: P) -> Self {
        CreateDatabase {
            transport: transport,
            db_path: db_path,
        }
    }

    pub fn run(self) -> Result<(), Error> {

        let db_name = try!(self.db_path.database_path());

        let response = try!(self.transport
                                .put(&[db_name.as_ref()],
                                     RequestOptions::new().with_accept_json()));

        match response.status_code() {
            StatusCode::Created => Ok(()),
            StatusCode::PreconditionFailed => Err(Error::database_exists(response)),
            StatusCode::Unauthorized => Err(Error::unauthorized(response)),
            _ => Err(Error::server_response(response)),
        }
    }
}

#[cfg(test)]
mod tests {

    use Error;
    use super::*;
    use transport::{MockRequestMatcher, MockResponse, MockTransport, StatusCode};

    #[test]
    fn create_database_ok() {

        let transport = MockTransport::new();
        transport.push_response(MockResponse::new(StatusCode::Created)
                                    .build_json_body(|x| x.insert("ok", true)));

        CreateDatabase::new(&transport, "/foo").run().unwrap();

        let expected = MockRequestMatcher::new().put(&["foo"], |x| x.with_accept_json());
        assert_eq!(expected, transport.extract_requests());
    }

    #[test]
    fn create_database_nok_database_exists() {

        let transport = MockTransport::new();
        let error = "file_exists";
        let reason = "The database could not be created, the file already exists.";
        transport.push_response(MockResponse::new(StatusCode::PreconditionFailed)
                                    .build_json_body(|x| {
                                        x.insert("error", error)
                                         .insert("reason", reason)
                                    }));

        match CreateDatabase::new(&transport, "/foo").run() {
            Err(Error::DatabaseExists(ref error_response)) if error == error_response.error() &&
                                                              reason == error_response.reason() => {
                ()
            }
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn create_database_nok_unauthorized() {

        let transport = MockTransport::new();
        let error = "unauthorized";
        let reason = "Authentication required.";
        transport.push_response(MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        }));

        match CreateDatabase::new(&transport, "/foo").run() {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
