use {Error, url};
use super::{AsyncActionHandler, JsonResponse, Request, ResponseHandler, ResponseHeaders, StatusCode, Transport};

pub struct JsonResponseBuilder {
    inner: JsonResponse,
}

impl JsonResponseBuilder {
    pub fn new(status_code: StatusCode) -> Self {
        JsonResponseBuilder {
            inner: JsonResponse {
                status_code: status_code,
                headers: ResponseHeaders::new(),
                content: Vec::new(),
            },
        }
    }

    pub fn unwrap(self) -> JsonResponse {
        self.inner
    }

    pub fn with_json_content_raw<S: AsRef<str>>(mut self, raw_json: S) -> Self {
        self.inner.content = raw_json.as_ref().bytes().collect();
        self
    }
}

pub struct MockTransport;

impl MockTransport {
    pub fn new() -> Self {
        MockTransport
    }
}

impl Transport for MockTransport {
    fn send<H, T>(&self, _request: Request, _response_handler: H) -> Result<T, Error>
        where H: ResponseHandler<T>
    {
        unimplemented!();
    }

    fn send_async<H, A, T, U>(&self, _request: Request, _response_handler: H, _action_handler: A) -> Result<U, Error>
        where A: AsyncActionHandler<T>,
              H: ResponseHandler<U>
    {
        unimplemented!();
    }

    fn make_base_url(&self) -> url::Url {
        url::Url::parse("http://example.com:5984").unwrap()
    }
}
