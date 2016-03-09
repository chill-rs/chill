use BasicDatabase;
use CreateDatabaseOptions;
use DatabaseName;
use Error;
use hyper;
use std;
use transport::{HyperTransport, RequestOptions, Response, StatusCode, Transport};

/// The `IntoUrl` trait applies to a type that is convertible into a URL.
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
        // FIXME: Eliminate this temporary.
        let db_name = String::from(db_name.into());

        let response = try!(self.transport
                                .put(&[&db_name], RequestOptions::new().with_accept_json()));

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
    use std;
    use super::BasicClient;
    use transport::{MockRequestMatcher, MockResponse, MockTransport, StatusCode};

    fn new_mock_client() -> BasicClient<MockTransport> {
        BasicClient { transport: std::sync::Arc::new(MockTransport::new()) }
    }

    #[test]
    fn create_database_ok() {

        let client = new_mock_client();
        client.transport.push_response(MockResponse::new(StatusCode::Created)
                                           .build_json_body(|x| x.insert("ok", true)));

        client.create_database("database_name", Default::default()).unwrap();

        let expected = MockRequestMatcher::new().put(&["database_name"], |x| x.with_accept_json());
        assert_eq!(expected, client.transport.extract_requests());

    }

    #[test]
    fn create_database_nok_database_exists() {

        let client = new_mock_client();
        let error = "file_exists";
        let reason = "The database could not be created, the file already exists.";
        client.transport.push_response(MockResponse::new(StatusCode::PreconditionFailed)
                                           .build_json_body(|x| {
                                               x.insert("error", error)
                                                .insert("reason", reason)
                                           }));

        match client.create_database("database_name", Default::default()) {
            Err(Error::DatabaseExists(ref error_response)) if error == error_response.error() &&
                                                              reason == error_response.reason() => {
                ()
            }
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn create_database_nok_unauthorized() {

        let client = new_mock_client();
        let error = "unauthorized";
        let reason = "Authentication required.";
        client.transport
              .push_response(MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
                  x.insert("error", error)
                   .insert("reason", reason)
              }));

        match client.create_database("database_name", Default::default()) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn select_database() {
        let db = {
            let client = new_mock_client();
            let db = client.select_database("database_name");
            assert_eq!(&DatabaseName::from("database_name"), db.name());
            db
        };

        assert_eq!(&DatabaseName::from("database_name"), db.name());
    }
}
