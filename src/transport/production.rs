use Error;
use error::TransportErrorKind;
use hyper;
use mime;
use serde;
use serde_json;
use std;
use super::{Action, Request, RequestMaker, Response};
use url;

pub struct Transport {
    server_url: url::Url,
    hyper_client: hyper::Client,
}

impl Transport {
    pub fn new(mut server_url: url::Url) -> Result<Self, Error> {

        if server_url.path_mut().is_none() {
            return Err(Error::UrlNotSchemeRelative);
        }

        // Sometimes the URL has an empty final path component, which will lead
        // to the wrong result if we append path components to the URL.

        {
            let mut path = server_url.path_mut().unwrap();
            if !path.is_empty() && path.last().unwrap().is_empty() {
                path.pop();
            }
        }

        Ok(Transport {
            hyper_client: hyper::Client::new(),
            server_url: server_url,
        })
    }

    #[cfg(test)]
    pub fn new_stub() -> Self {
        Transport::new("http://example.com:5984".parse().unwrap()).unwrap()
    }

    pub fn run_action<A>(&self, action: A) -> Result<A::Output, Error>
        where A: Action
    {
        let request_maker = HyperRequestMaker { transport: self };
        let (request, state) = try!(action.create_request(request_maker));
        let response = try!(request.send());
        A::handle_response(response, state)
    }
}

// NOTE: This implementation is a workaround for hyper::Client not implementing
// the Debug trait.
impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Transport {{ server_url: {:?} }}", self.server_url)
    }
}

struct HyperRequestMaker<'a> {
    transport: &'a Transport,
}

impl<'a> RequestMaker for HyperRequestMaker<'a> {
    type Request = HyperRequest<'a>;

    fn make_request<P>(&self,
                       method: hyper::method::Method,
                       url_path_components: P)
                       -> Self::Request
        where P: Iterator<Item = String>
    {
        let mut url = self.transport.server_url.clone();
        url.path_mut().unwrap().extend(url_path_components);

        HyperRequest {
            body: Vec::new(),
            headers: hyper::header::Headers::new(),
            method: method,
            url: url,
            transport: self.transport,
        }
    }
}

struct HyperRequest<'a> {
    transport: &'a Transport,
    method: hyper::method::Method,
    url: url::Url,
    headers: hyper::header::Headers,
    body: Vec<u8>,
}

impl<'a> HyperRequest<'a> {
    fn send(self) -> Result<HyperResponse, Error> {

        let response = try!(self.transport
                                .hyper_client
                                .request(self.method, self.url)
                                .headers(self.headers)
                                .body(&self.body[..])
                                .send()
                                .map_err(|e| {
                                    Error::Transport { kind: TransportErrorKind::Hyper(e) }
                                }));

        Ok(HyperResponse { hyper_response: response })
    }
}

impl<'a> Request for HyperRequest<'a> {
    fn set_content_type_json(mut self) -> Self {
        let mime = mime::Mime(mime::TopLevel::Application, mime::SubLevel::Json, vec![]);
        self.headers.set(hyper::header::ContentType(mime));
        self
    }

    fn set_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }
}

struct HyperResponse {
    hyper_response: hyper::client::Response,
}

impl Response for HyperResponse {
    fn status_code(&self) -> hyper::status::StatusCode {
        self.hyper_response.status
    }

    fn json_decode_content<T: serde::Deserialize>(self) -> Result<T, Error> {
        // FIXME: Return error if the Content-Type is not application/json.
        serde_json::from_reader(self.hyper_response).map_err(|e| Error::JsonDecode { cause: e })
    }
}

#[cfg(test)]
mod tests {

    use Error;
    use super::Transport;
    use url;

    #[test]
    fn new_nok_url_not_scheme_relative() {
        let url: url::Url = "foo:bar".parse().unwrap();
        let got = Transport::new(url);
        match got {
            Err(Error::UrlNotSchemeRelative) => (),
            Err(..) => panic!("Got unexpected error result {:?}", got),
            Ok(..) => panic!("Got unexpected OK result {:?}", got),
        }
    }

    #[test]
    fn new_ok_url_with_trailing_slash() {
        let url: url::Url = "http://example.com:5984/foo/".parse().unwrap();
        let transport = Transport::new(url).unwrap();
        assert_eq!(vec!["foo"], transport.server_url.path().unwrap());
    }
}
