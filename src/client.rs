use Database;
use DatabaseName;
use Error;
use hyper;
use std;
use transport::Transport;

/// Converts its type into a URL.
pub trait IntoUrl: hyper::client::IntoUrl {
}

impl<T: hyper::client::IntoUrl> IntoUrl for T {}

pub struct Client {
    transport: std::sync::Arc<Transport>,
}

impl Client {
    pub fn new<U: IntoUrl>(server_url: U) -> Result<Self, Error> {
        let server_url = try!(server_url.into_url().map_err(|e| Error::UrlParse { cause: e }));
        Ok(Client { transport: std::sync::Arc::new(try!(Transport::new(server_url))) })
    }

    pub fn select_database<D: Into<DatabaseName>>(&self, db_name: D) -> Database {
        Database::new(self.transport.clone(), db_name.into())
    }
}
