pub mod production;
#[cfg(test)]
pub mod testing;

use hyper;
use prelude_impl::*;
use serde;

pub trait Action<T: Transport> {
    type Output;
    type State;
    fn make_request(&mut self) -> Result<(T::Request, Self::State), Error>;
    fn take_response<R: Response>(response: R, state: Self::State) -> Result<Self::Output, Error>;
}

pub trait Transport {
    type Request;

    fn request<'a, B, P>(&self,
                         method: hyper::method::Method,
                         path: P,
                         options: RequestOptions<'a, B>)
                         -> Result<Self::Request, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>;

    fn delete<'a, B, P>(&self,
                        path: P,
                        options: RequestOptions<'a, B>)
                        -> Result<Self::Request, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        self.request(hyper::method::Method::Delete, path, options)
    }

    fn get<'a, B, P>(&self, path: P, options: RequestOptions<'a, B>) -> Result<Self::Request, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        self.request(hyper::method::Method::Get, path, options)
    }

    fn post<'a, B, P>(&self,
                      path: P,
                      options: RequestOptions<'a, B>)
                      -> Result<Self::Request, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        self.request(hyper::method::Method::Post, path, options)
    }

    fn put<'a, B, P>(&self, path: P, options: RequestOptions<'a, B>) -> Result<Self::Request, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        self.request(hyper::method::Method::Put, path, options)
    }
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
