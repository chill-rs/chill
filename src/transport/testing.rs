use Error;
use hyper;
use serde;
use serde_json;
use std;
use transport::{Request, RequestBuilder, Response, Transport};

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
// pre-constructed response. Hence, the typical test looks something like the
// following:
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
//
#[derive(Debug)]
pub struct MockTransport {
    requests: std::cell::RefCell<Vec<Request>>,
    responses: std::cell::RefCell<Vec<Response>>,
}

impl MockTransport {
    pub fn new() -> Self {
        MockTransport {
            requests: std::cell::RefCell::new(Vec::new()),
            responses: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub fn push_response(&self, response: Response) {
        self.responses.borrow_mut().push(response)
    }

    pub fn extract_requests(&self) -> Vec<Request> {
        use std::ops::DerefMut;
        std::mem::replace(self.requests.borrow_mut().deref_mut(), Vec::new())
    }
}

impl Transport for MockTransport {
    fn send(&self, request: Request) -> Result<Response, Error> {

        {
            let mut v = self.requests.borrow_mut();
            v.push(request);
        }

        let mut v = self.responses.borrow_mut();
        Ok(v.pop().unwrap())
    }
}

impl RequestBuilder {
    pub fn with_json_body_builder<F>(self, f: F) -> Self
        where F: FnOnce(serde_json::builder::ObjectBuilder) -> serde_json::builder::ObjectBuilder
    {
        let builder = serde_json::builder::ObjectBuilder::new();
        let body = f(builder).unwrap();
        self.with_json_body(&body)
    }
}

#[derive(Debug)]
pub struct ResponseBuilder {
    target_response: Response,
}

impl ResponseBuilder {
    pub fn new(status_code: hyper::status::StatusCode) -> Self {
        ResponseBuilder {
            target_response: Response {
                status_code: status_code,
                headers: hyper::header::Headers::new(),
                body: Vec::new(),
            },
        }
    }

    pub fn unwrap(self) -> Response {
        self.target_response
    }

    pub fn with_json_body<B: serde::Serialize>(mut self, body: &B) -> Self {

        let body = serde_json::to_vec(&body)
                       .map_err(|e| Error::JsonEncode { cause: e })
                       .unwrap();

        self.target_response.headers.set(hyper::header::ContentType(mime!(Application / Json)));
        self.target_response.body = body;
        self
    }

    #[cfg(test)]
    pub fn with_json_body_builder<F>(self, f: F) -> Self
        where F: FnOnce(serde_json::builder::ObjectBuilder) -> serde_json::builder::ObjectBuilder
    {
        let builder = serde_json::builder::ObjectBuilder::new();
        let body = f(builder).unwrap();
        self.with_json_body(&body)
    }
}

mod tests {

    use hyper;
    use serde_json;
    use super::ResponseBuilder;
    use transport::Response;

    #[test]
    fn response_builder_with_json_body() {

        let expected = Response {
            status_code: hyper::Ok,
            headers: {
                let mut headers = hyper::header::Headers::new();
                headers.set(hyper::header::ContentType(mime!(Application / Json)));
                headers
            },
            body: serde_json::to_vec(&serde_json::builder::ObjectBuilder::new()
                                          .insert("bar", 42)
                                          .unwrap())
                      .unwrap(),
        };

        let got = ResponseBuilder::new(hyper::Ok)
                      .with_json_body(&serde_json::builder::ObjectBuilder::new()
                                           .insert("bar", 42)
                                           .unwrap())
                      .unwrap();

        assert_eq!(expected, got);
    }
}
