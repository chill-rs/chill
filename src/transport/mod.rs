pub mod production;
#[cfg(test)]
pub mod testing;

use hyper;
use prelude_impl::*;
use serde;
use serde_json;

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
    attachments_query: Option<bool>,
    descending_query: Option<bool>,
    end_key_query: Option<String>,
    inclusive_end_query: Option<bool>,
    reduce_query: Option<bool>,
    revision_query: Option<&'a Revision>,
    start_key_query: Option<String>,
    body: Option<RequestBody<'a, B>>,
}

impl<'a> RequestOptions<'a, ()> {
    pub fn new() -> Self {
        RequestOptions::default()
    }

    pub fn with_json_body<B: serde::Serialize>(self, body: &'a B) -> RequestOptions<'a, B> {
        RequestOptions {
            accept: self.accept,
            attachments_query: self.attachments_query,
            descending_query: self.descending_query,
            end_key_query: self.end_key_query,
            inclusive_end_query: self.inclusive_end_query,
            reduce_query: self.reduce_query,
            revision_query: self.revision_query,
            start_key_query: self.start_key_query,
            body: Some(RequestBody::Json(body)),
        }
    }
}

impl<'a, B: serde::Serialize + 'a> RequestOptions<'a, B> {
    pub fn with_accept_json(mut self) -> Self {
        self.accept = Some(RequestAccept::Json);
        self
    }

    pub fn with_attachments_query(mut self, yes_or_no: bool) -> Self {
        self.attachments_query = Some(yes_or_no);
        self
    }

    pub fn with_descending_query(mut self, yes_or_no: bool) -> Self {
        self.descending_query = Some(yes_or_no);
        self
    }

    pub fn with_end_key<K: serde::Serialize>(mut self, key: &K) -> Result<Self, Error> {
        self.end_key_query = Some(try!(serde_json::to_string(key)
                                           .map_err(|e| Error::JsonEncode { cause: e })));
        Ok(self)
    }

    pub fn with_inclusive_end(mut self, yes_or_no: bool) -> Self {
        self.inclusive_end_query = Some(yes_or_no);
        self
    }

    pub fn with_reduce_query(mut self, yes_or_no: bool) -> Self {
        self.reduce_query = Some(yes_or_no);
        self
    }

    pub fn with_revision_query(mut self, revision: &'a Revision) -> Self {
        self.revision_query = Some(revision);
        self
    }

    pub fn with_start_key<K: serde::Serialize>(mut self, key: &K) -> Result<Self, Error> {
        self.start_key_query = Some(try!(serde_json::to_string(key)
                                             .map_err(|e| Error::JsonEncode { cause: e })));
        Ok(self)
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
