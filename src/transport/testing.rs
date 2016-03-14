use Error;
use Revision;
use serde;
use serde_json;
use std;
use super::{Method, RequestAccept, RequestBody, RequestOptions, Response, StatusCode, Transport};

// A mock transport allows us to test our CouchDB actions without the presence
// of a CouchDB server. This is helpful because:
//
//   * We get good test coverage even on a machine that doesn't have CouchDB
//     installed, and,
//
//   * We can test for different versions of CouchDB.
//
// The way it works is simple. A mock transport stores all incoming requests in
// a vector. For each incoming request, the transport responds with a
// pre-constructed response. Hence, the typical test works like this:
//
//   1. Construct all responses and add them to the mock transport.
//
//   2. Run the action being tested, which does the requesting and responding.
//      If the action produces correct results then (presumably) the action is
//      handling the response correctly.
//
//   3. Verify that the captured requests match the test's expectations.
//
// The three steps combine to ensure the action generates requests and handles
// responses correctly.

#[derive(Debug)]
pub struct MockTransport {
    requests: std::cell::RefCell<Vec<MockRequest>>,
    responses: std::cell::RefCell<Vec<MockResponse>>,
}

impl MockTransport {
    pub fn new() -> Self {
        MockTransport {
            requests: std::cell::RefCell::new(Vec::new()),
            responses: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub fn extract_requests(&self) -> Vec<MockRequest> {
        use std::ops::DerefMut;
        std::mem::replace(self.requests.borrow_mut().deref_mut(), Vec::new())
    }

    pub fn push_response(&self, response: MockResponse) {
        self.responses.borrow_mut().push(response)
    }

    fn request<'a, B, P>(&self,
                         method: Method,
                         path: P,
                         options: RequestOptions<'a, B>)
                         -> Result<<Self as Transport>::Response, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        self.requests.borrow_mut().push(MockRequest {
            method: method,
            path: path.into_iter().map(|x| x.to_string()).collect(),
            accept: match options.accept {
                None => None,
                Some(RequestAccept::Json) => Some(MockRequestAccept::Json),
            },
            revision_query: options.revision_query.map(|revision| revision.clone()),
            body: match options.body {
                None => None,
                Some(RequestBody::Json(body)) => {
                    Some(MockRequestBody::Json(serde_json::to_value(body)))
                }
            },
        });

        Ok(self.responses.borrow_mut().pop().unwrap())
    }
}

impl Transport for MockTransport {
    type Response = MockResponse;

    fn delete<'a, B, P>(&self,
                        path: P,
                        options: RequestOptions<'a, B>)
                        -> Result<Self::Response, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        self.request(Method::Delete, path, options)
    }

    fn get<'a, B, P>(&self,
                     path: P,
                     options: RequestOptions<'a, B>)
                     -> Result<Self::Response, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        self.request(Method::Get, path, options)
    }

    fn post<'a, B, P>(&self,
                      path: P,
                      options: RequestOptions<'a, B>)
                      -> Result<Self::Response, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        self.request(Method::Post, path, options)
    }

    fn put<'a, B, P>(&self,
                     path: P,
                     options: RequestOptions<'a, B>)
                     -> Result<Self::Response, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        self.request(Method::Put, path, options)
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
    method: Method,
    path: Vec<String>,
    accept: Option<MockRequestAccept>,
    revision_query: Option<Revision>,
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

#[derive(Debug)]
pub struct MockRequestMatcher {
    requests: Vec<MockRequest>,
}

impl MockRequestMatcher {
    pub fn new() -> Self {
        MockRequestMatcher { requests: Vec::new() }
    }

    pub fn delete<F>(mut self, path: &[&str], request_builder: F) -> Self
        where F: FnOnce(MockRequestBuilder) -> MockRequestBuilder
    {
        self.requests.push(request_builder(MockRequestBuilder::new(Method::Delete, path)).unwrap());
        self
    }

    pub fn get<F>(mut self, path: &[&str], request_builder: F) -> Self
        where F: FnOnce(MockRequestBuilder) -> MockRequestBuilder
    {
        self.requests.push(request_builder(MockRequestBuilder::new(Method::Get, path)).unwrap());
        self
    }

    pub fn post<F>(mut self, path: &[&str], request_builder: F) -> Self
        where F: FnOnce(MockRequestBuilder) -> MockRequestBuilder
    {
        self.requests.push(request_builder(MockRequestBuilder::new(Method::Post, path)).unwrap());
        self
    }

    pub fn put<F>(mut self, path: &[&str], request_builder: F) -> Self
        where F: FnOnce(MockRequestBuilder) -> MockRequestBuilder
    {
        self.requests.push(request_builder(MockRequestBuilder::new(Method::Put, path)).unwrap());
        self
    }
}

impl PartialEq<Vec<MockRequest>> for MockRequestMatcher {
    fn eq(&self, other: &Vec<MockRequest>) -> bool {
        self.requests == *other
    }
}

#[derive(Debug)]
pub struct MockRequestBuilder {
    target_request: MockRequest,
}

impl MockRequestBuilder {
    fn new(method: Method, path: &[&str]) -> Self {
        MockRequestBuilder {
            target_request: MockRequest {
                method: method,
                path: path.iter().map(|x| x.to_string()).collect(),
                accept: None,
                revision_query: None,
                body: None,
            },
        }
    }

    fn unwrap(self) -> MockRequest {
        self.target_request
    }

    pub fn with_accept_json(mut self) -> Self {
        self.target_request.accept = Some(MockRequestAccept::Json);
        self
    }

    pub fn with_revision_query(mut self, revision: &Revision) -> Self {
        self.target_request.revision_query = Some(revision.clone());
        self
    }

    pub fn with_json_body<B: serde::Serialize>(mut self, body: &B) -> Self {
        self.target_request.body = Some(MockRequestBody::Json(serde_json::to_value(body)));
        self
    }

    pub fn build_json_body<F>(self, f: F) -> Self
        where F: FnOnce(serde_json::builder::ObjectBuilder) -> serde_json::builder::ObjectBuilder
    {
        self.with_json_body(&f(serde_json::builder::ObjectBuilder::new()).unwrap())
    }
}
