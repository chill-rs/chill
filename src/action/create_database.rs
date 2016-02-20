use Database;
use DatabaseName;
use Error;
use error;
use hyper;
use std;
use transport::{Action, RequestMaker, Response, Transport};

struct ActionState {
    db_name: DatabaseName,
    transport: std::sync::Arc<Transport>,
}

pub struct CreateDatabase<'a> {
    transport: &'a std::sync::Arc<Transport>,
    db_name: DatabaseName,
}

impl<'a> CreateDatabase<'a> {
    #[doc(hidden)]
    pub fn new(transport: &'a std::sync::Arc<Transport>, db_name: DatabaseName) -> Self {
        CreateDatabase {
            db_name: db_name,
            transport: transport,
        }
    }

    pub fn run(self) -> Result<Database, Error> {
        self.transport.run_action(self)
    }
}

impl<'a> Action for CreateDatabase<'a> {
    type Output = Database;
    type State = ActionState;

    fn create_request<R>(self, request_maker: R) -> Result<(R::Request, Self::State), Error>
        where R: RequestMaker
    {
        let request = request_maker.make_request(hyper::method::Method::Put,
                                                 vec![self.db_name.clone()].into_iter());
        Ok((request,
            ActionState {
            db_name: self.db_name,
            transport: self.transport.clone(),
        }))
    }

    fn handle_response<R>(response: R, state: Self::State) -> Result<Self::Output, Error>
        where R: Response
    {
        use hyper::status::StatusCode;

        match response.status_code() {
            StatusCode::Created => Ok(Database::new(state.transport, state.db_name)),
            StatusCode::PreconditionFailed => Err(error::database_exists(response)),
            StatusCode::Unauthorized => Err(error::unauthorized(response)),
            _ => Err(error::unexpected_response(response)),
        }
    }
}

#[cfg(test)]
mod tests {

    use Error;
    use hyper;
    use std;
    use super::{ActionState, CreateDatabase};
    use transport::{Action, StubRequest, StubRequestMaker, StubResponse, Transport};

    #[test]
    fn create_request() {
        let expected_request = StubRequest::new(hyper::method::Method::Put,
                                                vec!["foo"]
                                                    .into_iter()
                                                    .map(|x| x.to_owned()));
        let transport = std::sync::Arc::new(Transport::new_stub());
        let action = CreateDatabase::new(&transport, "foo".to_owned());
        let (got_request, _) = action.create_request(StubRequestMaker::new())
                                     .unwrap();
        assert_eq!(expected_request, got_request);
    }

    #[test]
    fn handle_response_ok_created() {
        let response = StubResponse::new(hyper::status::StatusCode::Created);
        let state = ActionState {
            db_name: "foo".into(),
            transport: std::sync::Arc::new(Transport::new_stub()),
        };
        CreateDatabase::handle_response(response, state).unwrap();
        // FIXME: Check that the returned database is valid.
    }

    #[test]
    fn handle_response_nok_precondition_failed() {
        let error = "file_exists";
        let reason = "The database could not be created, the file already exists.";
        let response = StubResponse::new(hyper::status::StatusCode::PreconditionFailed)
                           .set_error_content(error, reason);
        let state = ActionState {
            db_name: "foo".into(),
            transport: std::sync::Arc::new(Transport::new_stub()),
        };
        let got = CreateDatabase::handle_response(response, state);
        expect_error_database_exists!(got, error, reason);
    }

    #[test]
    fn handle_response_nok_unauthorized() {
        let error = "unauthorized";
        let reason = "Authentication required.";
        let response = StubResponse::new(hyper::status::StatusCode::Unauthorized)
                           .set_error_content(error, reason);
        let state = ActionState {
            db_name: "foo".into(),
            transport: std::sync::Arc::new(Transport::new_stub()),
        };
        let got = CreateDatabase::handle_response(response, state);
        expect_error_unauthorized!(got, error, reason);
    }
}
