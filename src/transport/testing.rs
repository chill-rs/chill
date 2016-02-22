use Error;
use hyper;
use mime;
use serde;
use serde_json;
use super::{Request, RequestMaker, Response};

pub struct StubRequestMaker;

impl StubRequestMaker {
    pub fn new() -> Self {
        StubRequestMaker
    }
}

impl RequestMaker for StubRequestMaker {
    type Request = StubRequest;
    fn make_request<P>(&self,
                       method: hyper::method::Method,
                       url_path_components: P)
                       -> Self::Request
        where P: Iterator<Item = String>
    {
        StubRequest::new(method, url_path_components.collect::<Vec<_>>())
    }
}

#[derive(Debug, PartialEq)]
pub struct StubRequest {
    method: hyper::method::Method,
    url_path_components: Vec<String>,
    content_type: Option<mime::Mime>,
    body: Option<Vec<u8>>,
}

impl StubRequest {
    pub fn new<P, S>(method: hyper::method::Method, url_path_components: P) -> Self
        where P: IntoIterator<Item = S>,
              S: AsRef<str>
    {
        StubRequest {
            body: None,
            content_type: None,
            method: method,
            url_path_components: url_path_components.into_iter()
                                                    .map(|x| x.as_ref().into())
                                                    .collect(),
        }
    }
}

impl Request for StubRequest {
    fn set_content_type_json(mut self) -> Self {
        self.content_type = Some(mime::Mime(mime::TopLevel::Application,
                                            mime::SubLevel::Json,
                                            vec![]));
        self
    }

    fn set_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }
}

struct StubResponseContent {
    content_type: mime::Mime,
    content: Vec<u8>,
}

pub struct StubResponse {
    status_code: hyper::status::StatusCode,
    content: Option<StubResponseContent>,
}

impl StubResponse {
    pub fn new(status_code: hyper::status::StatusCode) -> Self {
        StubResponse {
            content: None,
            status_code: status_code,
        }
    }

    pub fn set_json_content<T: serde::Serialize>(mut self, content: &T) -> Self {

        use mime::{Mime, SubLevel, TopLevel};

        let mime = Mime(TopLevel::Application, SubLevel::Json, vec![]);
        let content = serde_json::to_vec(content)
                          .map_err(|e| Error::JsonEncode { cause: e })
                          .unwrap();

        self.content = Some(StubResponseContent {
            content_type: mime,
            content: content,
        });

        self
    }

    pub fn build_json_content<F>(self, builder: F) -> Self
        where F: FnOnce(serde_json::builder::ObjectBuilder) -> serde_json::builder::ObjectBuilder
    {
        let content = builder(serde_json::builder::ObjectBuilder::new()).unwrap();
        self.set_json_content(&content)
    }

    pub fn set_error_content<T, U>(self, error: T, reason: U) -> Self
        where T: AsRef<str>,
              U: AsRef<str>
    {
        let content = serde_json::builder::ObjectBuilder::new()
                          .insert("error", error.as_ref())
                          .insert("reason", reason.as_ref())
                          .unwrap();

        self.set_json_content(&content)
    }
}

impl Response for StubResponse {
    fn status_code(&self) -> hyper::status::StatusCode {
        self.status_code
    }

    fn json_decode_content<T: serde::Deserialize>(self) -> Result<T, Error> {

        use mime::{Mime, SubLevel, TopLevel};

        let content = self.content.expect("Response content is empty");

        if let Mime(TopLevel::Application, SubLevel::Json, _) = content.content_type {
        } else {
            panic!("Response content type is {}, not JSON",
                   content.content_type);
        }

        serde_json::from_reader(&content.content[..]).map_err(|e| Error::JsonDecode { cause: e })
    }
}
