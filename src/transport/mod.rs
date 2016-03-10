mod production;
#[cfg(test)]
mod testing;

use Error;
use Revision;
use serde;

pub use self::production::HyperTransport;
#[cfg(test)]
pub use self::testing::{MockRequestMatcher, MockResponse, MockTransport};

pub trait Transport {
    type Response: Response;

    fn delete<'a, B>(&self,
                     path: &[&str],
                     options: RequestOptions<'a, B>)
                     -> Result<Self::Response, Error>
        where B: serde::Serialize;

    fn get<'a, B>(&self,
                  path: &[&str],
                  options: RequestOptions<'a, B>)
                  -> Result<Self::Response, Error>
        where B: serde::Serialize;

    fn post<'a, B>(&self,
                   path: &[&str],
                   options: RequestOptions<'a, B>)
                   -> Result<Self::Response, Error>
        where B: serde::Serialize;

    fn put<'a, B>(&self,
                  path: &[&str],
                  options: RequestOptions<'a, B>)
                  -> Result<Self::Response, Error>
        where B: serde::Serialize;
}

pub trait Response {
    fn status_code(&self) -> StatusCode;
    fn decode_json_body<B: serde::Deserialize>(self) -> Result<B, Error>;
}

#[derive(Debug, Default)]
pub struct RequestOptions<'a, B: serde::Serialize + 'a> {
    accept: Option<RequestAccept>,
    revision_query: Option<&'a Revision>,
    body: Option<RequestBody<'a, B>>,
}

impl<'a> RequestOptions<'a, ()> {
    pub fn new() -> Self {
        RequestOptions::default()
    }

    pub fn with_json_body<B: serde::Serialize>(self, body: &'a B) -> RequestOptions<'a, B> {
        RequestOptions {
            accept: self.accept,
            revision_query: self.revision_query,
            body: Some(RequestBody::Json(body)),
        }
    }
}

impl<'a, B: serde::Serialize + 'a> RequestOptions<'a, B> {
    pub fn with_accept_json(mut self) -> Self {
        self.accept = Some(RequestAccept::Json);
        self
    }

    pub fn with_revision_query(mut self, revision: &'a Revision) -> Self {
        self.revision_query = Some(revision);
        self
    }
}

#[derive(Debug)]
enum RequestAccept {
    Json,
}

#[derive(Debug)]
enum RequestBody<'a, B: serde::Serialize + 'a> {
    Json(&'a B),
}

pub use hyper::method::Method;
pub use hyper::status::StatusCode;
