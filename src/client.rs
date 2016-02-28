use CreateDatabaseOptions;
use DatabaseName;
use database::BasicDatabase;
use Error;
use hyper;
use std;
use transport::{HyperTransport, RequestBuilder, Transport};

/// Converts its type into a URL.
pub trait IntoUrl: hyper::client::IntoUrl {
}

impl<T: hyper::client::IntoUrl> IntoUrl for T {}

pub type Client = BasicClient<HyperTransport>;

impl Client {
    pub fn new<U: IntoUrl>(server_url: U) -> Result<Self, Error> {
        let server_url = try!(server_url.into_url().map_err(|e| Error::UrlParse { cause: e }));
        let transport = try!(HyperTransport::new(server_url));
        Ok((BasicClient { transport: std::sync::Arc::new(transport) }))
    }
}

pub struct BasicClient<T: Transport> {
    transport: std::sync::Arc<T>,
}

impl<T: Transport> BasicClient<T> {
    pub fn create_database<D>(&self,
                              db_name: D,
                              _options: CreateDatabaseOptions)
                              -> Result<(), Error>
        where D: Into<DatabaseName>
    {
        use hyper::status::StatusCode;

        let request = RequestBuilder::new(hyper::method::Method::Put, vec![db_name.into()])
                          .with_accept_json()
                          .unwrap();

        let response = try!(self.transport.transport(request));

        match response.status_code() {
            StatusCode::Created => Ok(()),
            StatusCode::PreconditionFailed => Err(Error::database_exists(response)),
            StatusCode::Unauthorized => Err(Error::unauthorized(response)),
            _ => Err(Error::server_response(response)),
        }
    }

    pub fn select_database<D: Into<DatabaseName>>(&self, db_name: D) -> BasicDatabase<T> {
        BasicDatabase::new(self.transport.clone(), db_name.into())
    }
}

#[cfg(test)]
mod tests {

    use DatabaseName;
    use Error;
    use hyper;
    use std;
    use super::BasicClient;
    use transport::{Request, RequestBuilder, Response, ResponseBuilder, MockTransport};

    type MockClient = BasicClient<MockTransport>;

    impl MockClient {
        fn extract_requests(&self) -> Vec<Request> {
            self.transport.extract_requests()
        }

        fn push_response(&self, response: Response) {
            self.transport.push_response(response)
        }
    }

    fn new_mock_client() -> MockClient {
        MockClient { transport: std::sync::Arc::new(MockTransport::new()) }
    }

    #[test]
    fn create_database_ok() {

        let client = new_mock_client();
        client.push_response(ResponseBuilder::new(hyper::status::StatusCode::Created).unwrap());

        client.create_database("database_name", Default::default()).unwrap();

        let expected_requests = {
            vec![RequestBuilder::new(hyper::method::Method::Put,
                                     vec![String::from("database_name")])
                     .with_accept_json()
                     .unwrap()]
        };

        assert_eq!(expected_requests, client.extract_requests());
    }

    #[test]
    fn create_database_nok_database_exists() {

        let client = new_mock_client();
        let error = "file_exists";
        let reason = "The database could not be created, the file already exists.";
        client.push_response(ResponseBuilder::new(hyper::status::StatusCode::PreconditionFailed)
                                 .with_json_body_builder(|x| {
                                     x.insert("error", error)
                                      .insert("reason", reason)
                                 })
                                 .unwrap());

        match client.create_database("database_name", Default::default()) {
            Err(Error::DatabaseExists(ref error_response)) if error == error_response.error() &&
                                                              reason == error_response.reason() => {
                ()
            }
            e @ _ => unexpected_result!(e),
        }

        let expected_requests = {
            vec![RequestBuilder::new(hyper::method::Method::Put,
                                     vec![String::from("database_name")])
                     .with_accept_json()
                     .unwrap()]
        };

        assert_eq!(expected_requests, client.extract_requests());
    }

    #[test]
    fn create_database_nok_unauthorized() {

        let client = new_mock_client();
        let error = "unauthorized";
        let reason = "Authentication required.";
        client.push_response(ResponseBuilder::new(hyper::status::StatusCode::Unauthorized)
                                 .with_json_body_builder(|x| {
                                     x.insert("error", error)
                                      .insert("reason", reason)
                                 })
                                 .unwrap());

        match client.create_database("database_name", Default::default()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            e @ _ => unexpected_result!(e),
        }

        let expected_requests = {
            vec![RequestBuilder::new(hyper::method::Method::Put,
                                     vec![String::from("database_name")])
                     .with_accept_json()
                     .unwrap()]
        };

        assert_eq!(expected_requests, client.extract_requests());
    }

    #[test]
    fn select_database() {
        let client = new_mock_client();
        let db = client.select_database("database_name");
        assert_eq!(&DatabaseName::from("database_name"), db.name());
    }
}
