use Error;
use hyper;
use Revision;
use serde;
use serde_json;
use super::{RequestAccept, RequestBody, RequestOptions, Response, StatusCode, Transport};

// A mock transport allows us to test our CouchDB actions without the presence
// of a CouchDB server. This is helpful because:
//
//   * We get good test coverage even on a machine that doesn't have CouchDB
//     installed, and,
//
//   * We can test for different versions of CouchDB.

#[derive(Debug)]
pub struct MockTransport;

impl MockTransport {
    pub fn new() -> Self {
        MockTransport
    }
}

impl Transport for MockTransport {
    type Request = MockRequest;

    fn request<'a, B, P>(&self,
                         method: hyper::method::Method,
                         path: P,
                         options: RequestOptions<'a, B>)
                         -> Result<Self::Request, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        Ok(MockRequest {
            method: method,
            path: path.into_iter().map(|x| x.to_string()).collect(),
            accept: match options.accept {
                None => None,
                Some(RequestAccept::Json) => Some(MockRequestAccept::Json),
            },
            attachments_query: options.attachments_query,
            descending_query: options.descending_query,
            end_key_query: options.end_key_query,
            group_query: options.group_query,
            group_level_query: options.group_level_query,
            inclusive_end_query: options.inclusive_end_query,
            limit: options.limit,
            reduce_query: options.reduce_query,
            revision_query: options.revision_query.map(|revision| revision.clone()),
            start_key_query: options.start_key_query,
            body: match options.body {
                None => None,
                Some(RequestBody::Json(body)) => {
                    Some(MockRequestBody::Json(serde_json::to_value(body)))
                }
            },
        })
    }
}

#[derive(Debug)]
pub struct MockResponse {
    status_code: StatusCode,
    body: Option<MockResponseBody>,
}

#[derive(Debug)]
enum MockResponseBody {
    Json(serde_json::Value),
}

impl MockResponse {
    pub fn new(status_code: StatusCode) -> Self {
        MockResponse {
            status_code: status_code,
            body: None,
        }
    }

    pub fn with_json_body<B: serde::Serialize>(mut self, body: B) -> Self {
        self.body = Some(MockResponseBody::Json(serde_json::to_value(&body)));
        self
    }

    pub fn build_json_body<F>(self, f: F) -> Self
        where F: FnOnce(serde_json::builder::ObjectBuilder) -> serde_json::builder::ObjectBuilder
    {
        self.with_json_body(f(serde_json::builder::ObjectBuilder::new()).unwrap())
    }
}

impl Response for MockResponse {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn decode_json_body<B: serde::Deserialize>(self) -> Result<B, Error> {
        match self.body {
            None => Err(Error::ResponseNotJson(None)),
            Some(MockResponseBody::Json(body)) => {
                serde_json::from_value(body).map_err(|e| Error::JsonDecode { cause: e })
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct MockRequest {
    method: hyper::method::Method,
    path: Vec<String>,
    accept: Option<MockRequestAccept>,
    attachments_query: Option<bool>,
    descending_query: Option<bool>,
    end_key_query: Option<String>,
    group_query: Option<bool>,
    group_level_query: Option<u32>,
    inclusive_end_query: Option<bool>,
    limit: Option<u64>,
    reduce_query: Option<bool>,
    revision_query: Option<Revision>,
    start_key_query: Option<String>,
    body: Option<MockRequestBody>,
}

#[derive(Debug, PartialEq)]
enum MockRequestAccept {
    Json,
}

#[derive(Debug, PartialEq)]
enum MockRequestBody {
    Json(serde_json::Value),
}
