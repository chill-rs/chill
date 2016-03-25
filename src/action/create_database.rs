use DatabasePathRef;
use Error;
use IntoDatabasePath;
use transport::{Action, HyperTransport, RequestOptions, Response, StatusCode, Transport};

pub struct CreateDatabase<'a, T: Transport + 'a> {
    transport: &'a T,
    db_path: DatabasePathRef<'a>,
}

impl<'a, T: Transport + 'a> CreateDatabase<'a, T> {
    #[doc(hidden)]
    pub fn new<P: IntoDatabasePath<'a>>(transport: &'a T, db_path: P) -> Result<Self, Error> {
        Ok(CreateDatabase {
            transport: transport,
            db_path: try!(db_path.into_database_path()),
        })
    }
}

impl<'a> CreateDatabase<'a, HyperTransport> {
    pub fn run(self) -> Result<<Self as Action<HyperTransport>>::Output, Error> {
        self.transport.exec_sync(self)
    }
}

impl<'a, T: Transport + 'a> Action<T> for CreateDatabase<'a, T> {
    type Output = ();
    type State = ();

    fn make_request(&mut self) -> Result<(T::Request, Self::State), Error> {
        let options = RequestOptions::new().with_accept_json();
        let request = try!(self.transport.put(self.db_path, options));
        Ok((request, ()))
    }

    fn take_response<R: Response>(response: R, _state: Self::State) -> Result<Self::Output, Error> {
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
    use transport::{Action, MockResponse, MockTransport, RequestOptions, StatusCode, Transport};

    #[test]
    fn make_request_default() {
        let transport = MockTransport::new();

        let expected = ({
            let options = RequestOptions::new().with_accept_json();
            transport.put(vec!["foo"], options).unwrap()
        },
                        ());

        let got = {
            let mut action = CreateDatabase::new(&transport, "/foo").unwrap();
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_created() {
        let response = MockResponse::new(StatusCode::Created)
                           .build_json_body(|x| x.insert("ok", true));
        let expected = ();
        let got = CreateDatabase::<MockTransport>::take_response(response, ()).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_precondition_failed() {
        let error = "file_exists";
        let reason = "The database could not be created, the file already exists.";
        let response = MockResponse::new(StatusCode::PreconditionFailed).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        });
        match CreateDatabase::<MockTransport>::take_response(response, ()) {
            Err(Error::DatabaseExists(ref error_response)) if error == error_response.error() &&
                                                              reason == error_response.reason() => {
                ()
            }
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
        match CreateDatabase::<MockTransport>::take_response(response, ()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
