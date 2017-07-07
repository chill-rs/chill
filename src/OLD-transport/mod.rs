#[cfg(test)]
mod testing;


#[cfg(test)]
pub use self::testing::{JsonResponseBuilder, MockTransport};
use {Error, hyper, serde, serde_json, std, url};
use error::TransportErrorKind;
pub use hyper::method::Method;
pub use hyper::status::StatusCode;
use std::io::prelude::*;

pub trait AsQueryKey {
    type Key: AsRef<str>;
    fn as_query_key(&self) -> Self::Key;
}

pub trait AsQueryValue<K: AsQueryKey> {
    type Value: AsRef<str>;
    fn as_query_value(&self) -> Self::Value;
}

pub trait AsQueryValueFallible<K: AsQueryKey> {
    type Value: AsRef<str>;
    fn as_query_value_fallible(&self) -> Result<Self::Value, Error>;
}

#[derive(Debug, PartialEq)]
pub struct Request {
    method: hyper::method::Method,
    url: url::Url,
    headers: hyper::header::Headers,
    body: Vec<u8>,
}

impl Request {
    pub fn new(method: hyper::method::Method, url: url::Url) -> Self {
        Request {
            method: method,
            url: url,
            headers: hyper::header::Headers::new(),
            body: Vec::new(),
        }
    }

    pub fn with_accept_json(mut self) -> Self {
        let quality_items = vec![hyper::header::qitem(mime!(Application / Json))];
        self.headers.set(hyper::header::Accept(quality_items));
        self
    }

    pub fn with_json_content<C: serde::Serialize>(mut self, content: &C) -> Result<Self, Error> {
        self.headers.set(hyper::header::ContentType(
            mime!(Application / Json),
        ));
        self.body = try!(serde_json::to_vec(content).map_err(|e| {
            Error::JsonEncode { cause: e }
        }));
        Ok(self)
    }

    pub fn with_query<K, V>(mut self, key: K, value: &V) -> Self
    where
        K: AsQueryKey,
        V: AsQueryValue<K>,
    {
        self.url.query_pairs_mut().append_pair(
            key.as_query_key().as_ref(),
            value.as_query_value().as_ref(),
        );
        self
    }

    pub fn with_query_fallible<K, V>(mut self, key: K, value: &V) -> Result<Self, Error>
    where
        K: AsQueryKey,
        V: AsQueryValueFallible<K>,
    {
        self.url.query_pairs_mut().append_pair(
            key.as_query_key().as_ref(),
            try!(value.as_query_value_fallible()).as_ref(),
        );
        Ok(self)
    }

    #[cfg(test)]
    pub fn with_query_literal<K, V>(mut self, key: K, value: V) -> Self
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.url.query_pairs_mut().append_pair(
            key.as_ref(),
            value.as_ref(),
        );
        self
    }
}

pub trait ResponseHandler<T> {
    fn handle_response_status_and_headers(
        &mut self,
        status_code: StatusCode,
        headers: ResponseHeaders,
    ) -> Result<(), Error>;
    fn handle_response_content(&mut self, content: Vec<u8>) -> Result<(), Error>;
    fn handle_response_eof(self) -> Result<T, Error>;
}

pub trait JsonResponseHandler<T> {
    fn handle_json_response(self, response: JsonResponse) -> Result<T, Error>;
}

impl<F, T> JsonResponseHandler<T> for F
where
    F: FnOnce(JsonResponse) -> Result<T, Error>,
{
    fn handle_json_response(self, response: JsonResponse) -> Result<T, Error> {
        self(response)
    }
}

pub struct JsonResponseDecoder<H, T>
where
    H: JsonResponseHandler<T>,
{
    handler: H,
    status_code: StatusCode,
    headers: ResponseHeaders,
    content: Vec<u8>,
    _phantom: std::marker::PhantomData<T>,
}

impl<H, T> JsonResponseDecoder<H, T>
where
    H: JsonResponseHandler<T>,
{
    pub fn new(handler: H) -> Self {
        JsonResponseDecoder {
            handler: handler,
            status_code: StatusCode::Ok,
            headers: ResponseHeaders::new(),
            content: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<H, T> ResponseHandler<T> for JsonResponseDecoder<H, T>
where
    H: JsonResponseHandler<T>,
{
    fn handle_response_status_and_headers(
        &mut self,
        status_code: StatusCode,
        mut headers: ResponseHeaders,
    ) -> Result<(), Error> {

        try!(headers.extract_content_type_as_json());

        self.status_code = status_code;
        self.headers = headers;
        Ok(())
    }

    fn handle_response_content(&mut self, content: Vec<u8>) -> Result<(), Error> {
        self.content.extend_from_slice(&content);
        Ok(())
    }

    fn handle_response_eof(mut self) -> Result<T, Error> {
        self.handler.handle_json_response(JsonResponse {
            status_code: self.status_code,
            headers: std::mem::replace(&mut self.headers, ResponseHeaders::new()),
            content: std::mem::replace(&mut self.content, Vec::new()),
        })
    }
}

pub struct ResponseHeaders {
    headers: hyper::header::Headers,
}

impl ResponseHeaders {
    pub fn new() -> Self {
        ResponseHeaders { headers: hyper::header::Headers::new() }
    }

    fn extract_content_type_as_json(&mut self) -> Result<(), Error> {

        use hyper::header::ContentType;
        use mime::{Mime, SubLevel, TopLevel};

        match self.headers.get::<ContentType>() {
            Some(&ContentType(Mime(TopLevel::Application, SubLevel::Json, _))) => (),
            Some(&ContentType(ref mime)) => {
                return Err(Error::ResponseNotJson(Some(mime.clone())));
            }
            None => {
                return Err(Error::ResponseNotJson(None));
            }
        }

        self.headers.remove::<ContentType>();

        Ok(())
    }
}

impl From<hyper::header::Headers> for ResponseHeaders {
    fn from(x: hyper::header::Headers) -> Self {
        ResponseHeaders { headers: x }
    }
}

pub struct JsonResponse {
    status_code: StatusCode,
    headers: ResponseHeaders,
    content: Vec<u8>,
}

impl JsonResponse {
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    pub fn headers(&self) -> &ResponseHeaders {
        &self.headers
    }

    pub fn decode_content<T: serde::Deserialize>(&self) -> Result<T, Error> {
        serde_json::from_slice(&self.content).map_err(|e| Error::JsonDecode { cause: e })
    }
}

pub trait Transport {
    fn send<H, T>(&self, request: Request, response_handler: H) -> Result<T, Error>
    where
        H: ResponseHandler<T>;
    fn send_async<H, A, T, U>(&self, request: Request, response_handler: H, action_handler: A) -> Result<U, Error>
    where
        A: AsyncActionHandler<T>,
        H: ResponseHandler<U>;

    fn make_base_url(&self) -> url::Url;

    fn request<P>(&self, method: hyper::method::Method, path_segments: P) -> Request
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        let url = {
            let mut u = self.make_base_url();
            u.path_segments_mut()
                .expect("Server URL is not cannot-be-base")
                .extend(path_segments);
            u
        };

        Request::new(method, url)
    }

    fn delete<P>(&self, path_segments: P) -> Request
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        self.request(hyper::method::Method::Delete, path_segments)
    }

    fn get<P>(&self, path_segments: P) -> Request
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        self.request(hyper::method::Method::Get, path_segments)
    }

    fn post<P>(&self, path_segments: P) -> Request
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        self.request(hyper::method::Method::Post, path_segments)
    }

    fn put<P>(&self, path_segments: P) -> Request
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        self.request(hyper::method::Method::Put, path_segments)
    }
}

pub trait AsyncActionHandler<T> {
    fn handle(self, result: Result<T, Error>);
}

impl<F, T> AsyncActionHandler<T> for F
where
    F: FnOnce(Result<T, Error>),
{
    fn handle(self, result: Result<T, Error>) {
        self(result)
    }
}

#[derive(Debug)]
pub struct HyperTransport {
    server_base_url: url::Url,
    hyper_client: hyper::Client,
}

impl HyperTransport {
    pub fn new(server_base_url: url::Url) -> Self {
        HyperTransport {
            server_base_url: server_base_url,
            hyper_client: hyper::Client::new(),
        }
    }
}

impl Transport for HyperTransport {
    fn send<H, T>(&self, request: Request, mut response_handler: H) -> Result<T, Error>
    where
        H: ResponseHandler<T>,
    {
        let mut response = {
            let requester = self.hyper_client
                .request(request.method, request.url)
                .headers(request.headers);

            let requester = if request.body.is_empty() {
                requester
            } else {
                requester.body(&request.body[..])
            };

            try!(requester.send().map_err(|e| {
                Error::Transport { kind: TransportErrorKind::Hyper(e) }
            }))
        };

        let headers = std::mem::replace(&mut response.headers, hyper::header::Headers::new());
        let headers = ResponseHeaders::from(headers);
        try!(response_handler.handle_response_status_and_headers(
            response.status,
            headers,
        ));

        let mut body = Vec::new();
        try!(response.read_to_end(&mut body).map_err(|e| {
            Error::Io {
                cause: e,
                description: "Failed to read response from server",
            }
        }));

        try!(response_handler.handle_response_content(body));
        response_handler.handle_response_eof()
    }

    fn send_async<H, A, T, U>(&self, _request: Request, _response_handler: H, _action_handler: A) -> Result<U, Error>
    where
        A: AsyncActionHandler<T>,
        H: ResponseHandler<U>,
    {
        unimplemented!();
    }

    fn make_base_url(&self) -> url::Url {
        self.server_base_url.clone()
    }
}
