use Error;
use hyper;
use mime;
use serde;
use serde_json;
use std;
use super::{RequestMaker, Response};

#[cfg(test)]
pub struct StubRequestMaker;

#[cfg(test)]
impl StubRequestMaker {
    pub fn new() -> Self {
        StubRequestMaker
    }
}

#[cfg(test)]
impl RequestMaker for StubRequestMaker {
    type Request = StubRequest;
    fn make_request<P>(&self,
                       method: hyper::method::Method,
                       url_path_components: P)
                       -> Self::Request
        where P: Iterator<Item = String>
    {
        StubRequest {
            method: method,
            url_path_components: url_path_components.collect(),
        }
    }
}

#[cfg(test)]
#[derive(Debug, Eq, PartialEq)]
pub struct StubRequest {
    method: hyper::method::Method,
    url_path_components: std::collections::HashSet<String>,
}

#[cfg(test)]
impl StubRequest {
    pub fn new<P>(method: hyper::method::Method, url_path_components: P) -> Self
        where P: Iterator<Item = String>
    {
        StubRequest {
            method: method,
            url_path_components: url_path_components.collect(),
        }
    }
}

#[cfg(test)]
struct StubResponseContent {
    content_type: mime::Mime,
    content: Vec<u8>,
}

#[cfg(test)]
pub struct StubResponse {
    status_code: hyper::status::StatusCode,
    content: Option<StubResponseContent>,
}

#[cfg(test)]
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

#[cfg(test)]
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
