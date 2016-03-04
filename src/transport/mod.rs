mod production;
#[cfg(test)]
mod testing;

use Error;
use error::TransportErrorKind;
use hyper;
use Revision;
use serde;
use serde_json;
use std;
use std::io::prelude::*;

pub use self::production::HyperTransport;
#[cfg(test)]
pub use self::testing::{MockTransport, ResponseBuilder};

pub trait Transport {
    fn send(&self, request: Request) -> Result<Response, Error>;
}

#[derive(Debug, PartialEq)]
pub struct Request {
    method: hyper::method::Method,
    path: Vec<String>,
    query: std::collections::HashMap<String, String>,
    headers: hyper::header::Headers,
    body: PrintableBytes,
}

#[derive(Debug)]
pub struct RequestBuilder {
    target_request: Request,
}

impl RequestBuilder {
    pub fn new(method: hyper::method::Method, path: Vec<String>) -> Self {
        RequestBuilder {
            target_request: Request {
                method: method,
                path: path,
                query: std::collections::HashMap::new(),
                headers: hyper::header::Headers::new(),
                body: PrintableBytes(Vec::new()),
            },
        }
    }

    pub fn unwrap(self) -> Request {
        self.target_request
    }

    pub fn with_accept_json(mut self) -> Self {

        debug_assert!(self.target_request.headers.get::<hyper::header::Accept>().is_none());

        let quality_items = vec![hyper::header::qitem(mime!(Application / Json))];
        self.target_request.headers.set(hyper::header::Accept(quality_items));

        self
    }

    pub fn with_json_body<B: serde::Serialize>(mut self, body: &B) -> Self {

        let body = serde_json::to_vec(&body)
                       .map_err(|e| Error::JsonEncode { cause: e })
                       .unwrap();

        self.target_request.headers.set(hyper::header::ContentType(mime!(Application / Json)));
        self.target_request.body = PrintableBytes(body);
        self
    }

    pub fn with_revision_query(mut self, revision: &Revision) -> Self {
        self.target_request.query.insert("rev".to_string(), revision.to_string());
        self
    }
}

#[derive(Debug, PartialEq)]
pub struct Response {
    status_code: hyper::status::StatusCode,
    headers: hyper::header::Headers,
    body: Vec<u8>,
}

impl Response {
    pub fn status_code(&self) -> hyper::status::StatusCode {
        self.status_code
    }

    pub fn decode_json_body<B: serde::Deserialize>(self) -> Result<B, Error> {

        use hyper::header::ContentType;
        use mime::{Mime, SubLevel, TopLevel};

        try!(match self.headers.get() {
            Some(&ContentType(Mime(TopLevel::Application, SubLevel::Json, _))) => Ok(()),
            Some(&ContentType(ref mime)) => Err(Error::ResponseNotJson(Some(mime.clone()))),
            None => Err(Error::ResponseNotJson(None)),
        });

        serde_json::from_slice(&self.body[..]).map_err(|e| Error::JsonDecode { cause: e })
    }

    pub fn from_hyper_response(mut hyper_response: hyper::client::Response) -> Result<Self, Error> {

        let body = {
            let mut body = Vec::<u8>::new();
            try!(hyper_response.read_to_end(&mut body)
                               .map_err(|e| Error::Transport { kind: TransportErrorKind::Io(e) }));
            body
        };

        Ok(Response {
            status_code: hyper_response.status,
            headers: std::mem::replace(&mut hyper_response.headers, hyper::header::Headers::new()),
            body: body,
        })
    }
}

#[derive(PartialEq)]
struct PrintableBytes(Vec<u8>);

impl std::fmt::Debug for PrintableBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let &PrintableBytes(ref bytes) = self;
        match String::from_utf8(bytes.clone()) {
            Ok(s) => s.fmt(f),
            Err(_) => bytes.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {

    use hyper;
    use serde_json;
    use std;
    use super::{PrintableBytes, Request, RequestBuilder};

    #[test]
    fn request_builder_with_accept_json() {

        let expected = Request {
            method: hyper::Get,
            path: vec![String::from("foo")],
            query: std::collections::HashMap::new(),
            headers: {
                let mut headers = hyper::header::Headers::new();
                let quality_items = vec![hyper::header::qitem(mime!(Application / Json))];
                headers.set(hyper::header::Accept(quality_items));
                headers
            },
            body: PrintableBytes(Vec::new()),
        };

        let got = RequestBuilder::new(hyper::Get, vec![String::from("foo")])
                      .with_accept_json()
                      .unwrap();

        assert_eq!(expected, got);
    }

    #[test]
    fn request_builder_with_json_body() {

        let expected = Request {
            method: hyper::Get,
            path: vec![String::from("foo")],
            query: std::collections::HashMap::new(),
            headers: {
                let mut headers = hyper::header::Headers::new();
                headers.set(hyper::header::ContentType(mime!(Application / Json)));
                headers
            },
            body: PrintableBytes(serde_json::to_vec(&serde_json::builder::ObjectBuilder::new()
                                                         .insert("bar", 42)
                                                         .unwrap())
                                     .unwrap()),
        };

        let got = RequestBuilder::new(hyper::Get, vec![String::from("foo")])
                      .with_json_body(&serde_json::builder::ObjectBuilder::new()
                                           .insert("bar", 42)
                                           .unwrap())
                      .unwrap();

        assert_eq!(expected, got);
    }

    #[test]
    fn request_builder_with_revision_query() {

        let expected = Request {
            method: hyper::Get,
            path: vec![],
            query: vec![("rev".to_string(),
                         "1-1234567890abcdef1234567890abcdef".to_string())]
                       .into_iter()
                       .collect(),
            headers: hyper::header::Headers::new(),
            body: PrintableBytes(Vec::new()),
        };

        let rev = "1-1234567890abcdef1234567890abcdef".parse().unwrap();
        let got = RequestBuilder::new(hyper::Get, vec![])
                      .with_revision_query(&rev)
                      .unwrap();

        assert_eq!(expected, got);
    }
}
