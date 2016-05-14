use {Error, IntoDatabasePath};
use transport::{Action, RequestOptions, Response, StatusCode, Transport};
use transport::production::HyperTransport;

pub struct CreateDatabase<'a, T: Transport + 'a, P: IntoDatabasePath> {
    transport: &'a T,
    db_path: P,
}

impl<'a, P: IntoDatabasePath, T: Transport + 'a> CreateDatabase<'a, T, P> {
    #[doc(hidden)]
    pub fn new(transport: &'a T, db_path: P) -> Self {
        CreateDatabase {
            transport: transport,
            db_path: db_path,
        }
    }
}

impl<'a, P: IntoDatabasePath> CreateDatabase<'a, HyperTransport, P> {
    pub fn run(self) -> Result<(), Error> {
        self.transport.exec_sync(self)
    }
}

impl<'a, P: IntoDatabasePath, T: Transport + 'a> Action<T> for CreateDatabase<'a, T, P> {
    type Output = ();
    type State = ();

    fn make_request(self) -> Result<(T::Request, Self::State), Error> {
        let db_path = try!(self.db_path.into_database_path());
        let options = RequestOptions::new().with_accept_json();
        let request = try!(self.transport.put(db_path.iter(), options));
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

    use {DatabasePath, Error};
    use super::CreateDatabase;
    use transport::{Action, RequestOptions, StatusCode, Transport};
    use transport::testing::{MockResponse, MockTransport};

    #[test]
    fn make_request_default() {
        let transport = MockTransport::new();

        let expected = ({
            let options = RequestOptions::new().with_accept_json();
            transport.put(vec!["foo"], options).unwrap()
        },
                        ());

        let got = {
            let action = CreateDatabase::new(&transport, "/foo");
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_created() {
        let response = MockResponse::new(StatusCode::Created).build_json_body(|x| x.insert("ok", true));
        let expected = ();
        let got = CreateDatabase::<MockTransport, DatabasePath>::take_response(response, ()).unwrap();
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
        match CreateDatabase::<MockTransport, DatabasePath>::take_response(response, ()) {
            Err(Error::DatabaseExists(ref error_response)) if error == error_response.error() &&
                                                              reason == error_response.reason() => (),
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
        match CreateDatabase::<MockTransport, DatabasePath>::take_response(response, ()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
