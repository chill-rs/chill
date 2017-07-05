mod net;

pub use self::net::NetTransport;
use Error;
use futures::Future;
pub use reqwest::{Method, StatusCode};

pub trait Transport {
    type Request: Request;
    fn request(&self, method: Method, path: &str) -> Self::Request;
}

pub trait Request {
    type Response: Response + 'static;
    fn send(self) -> Box<Future<Item = Self::Response, Error = Error>>;
}

pub trait Response {
    fn status_code(&self) -> StatusCode;

    fn into_error(&self) -> Error {
        Error::NokResponse {
            status_code: self.status_code(),
            body: None, // FIXME: Decode JSON body.
        }
    }
}
