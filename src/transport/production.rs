use Error;
use error::TransportErrorKind;
use hyper;
use serde;
use serde_json;
use std;
use super::{Method, RequestAccept, RequestBody, RequestOptions, Response, StatusCode, Transport};
use url;

pub struct HyperTransport {
    server_base_url: url::Url,
    hyper_client: hyper::Client,
}

impl HyperTransport {
    pub fn new(mut server_base_url: url::Url) -> Result<Self, Error> {

        if server_base_url.path_mut().is_none() {
            return Err(Error::UrlNotSchemeRelative);
        }

        // Sometimes the URL has an empty final path component, which will lead
        // to an empty path component (//) if we naively append path components
        // to the URL. Remove any empty final component now.

        {
            let mut path = server_base_url.path_mut().unwrap();
            if !path.is_empty() && path.last().unwrap().is_empty() {
                path.pop();
            }
        }

        Ok(HyperTransport {
            server_base_url: server_base_url,
            hyper_client: hyper::Client::new(),
        })
    }

    fn request<'a, B>(&self,
                      method: Method,
                      path: &[&str],
                      options: RequestOptions<'a, B>)
                      -> Result<<Self as Transport>::Response, Error>
        where B: serde::Serialize
    {
        let uri = {
            let mut u = self.server_base_url.clone();
            u.path_mut().unwrap().extend(path.iter().map(|x| x.to_string()));
            u.set_query_from_pairs({
                let mut pairs = std::collections::HashMap::new();
                if let Some(revision) = options.revision_query {
                    pairs.insert("rev".to_string(), revision.to_string());
                }
                pairs
            });
            u
        };

        let headers = {
            let mut h = hyper::header::Headers::new();

            match options.accept {
                None => (),
                Some(RequestAccept::Json) => {
                    let quality_items = vec![hyper::header::qitem(mime!(Application / Json))];
                    h.set(hyper::header::Accept(quality_items));
                }
            }

            match options.body {
                None => (),
                Some(RequestBody::Json(..)) => {
                    h.set(hyper::header::ContentType(mime!(Application / Json)));
                }
            }

            h
        };

        Ok(HyperResponse {
            hyper_response: try!({
                match options.body {
                    None => {
                        self.hyper_client
                            .request(method, uri)
                            .headers(headers)
                            .send()
                    }
                    Some(RequestBody::Json(body)) => {
                        let body = try!(serde_json::to_vec(body)
                                            .map_err(|e| Error::JsonEncode { cause: e }));
                        self.hyper_client
                            .request(method, uri)
                            .headers(headers)
                            .body(&body[..])
                            .send()
                    }
                }
                .map_err(|e| Error::Transport { kind: TransportErrorKind::Hyper(e) })
            }),
        })
    }
}

impl Transport for HyperTransport {
    type Response = HyperResponse;

    fn get<'a, B>(&self,
                  path: &[&str],
                  options: RequestOptions<'a, B>)
                  -> Result<Self::Response, Error>
        where B: serde::Serialize
    {
        self.request(Method::Get, path, options)
    }

    fn post<'a, B>(&self,
                   path: &[&str],
                   options: RequestOptions<'a, B>)
                   -> Result<Self::Response, Error>
        where B: serde::Serialize
    {
        self.request(Method::Post, path, options)
    }

    fn put<'a, B>(&self,
                  path: &[&str],
                  options: RequestOptions<'a, B>)
                  -> Result<Self::Response, Error>
        where B: serde::Serialize
    {
        self.request(Method::Put, path, options)
    }
}

#[derive(Debug)]
pub struct HyperResponse {
    hyper_response: hyper::client::Response,
}

impl Response for HyperResponse {
    fn status_code(&self) -> StatusCode {
        self.hyper_response.status
    }

    fn decode_json_body<B: serde::Deserialize>(self) -> Result<B, Error> {

        use hyper::header::ContentType;
        use mime::{Mime, SubLevel, TopLevel};

        try!(match self.hyper_response.headers.get() {
            Some(&ContentType(Mime(TopLevel::Application, SubLevel::Json, _))) => Ok(()),
            Some(&ContentType(ref mime)) => Err(Error::ResponseNotJson(Some(mime.clone()))),
            None => Err(Error::ResponseNotJson(None)),
        });

        serde_json::from_reader(self.hyper_response).map_err(|e| Error::JsonDecode { cause: e })
    }
}

#[cfg(test)]
mod tests {

    use super::HyperTransport;

    #[test]
    fn new_hyper_transport_sanitizes_base_url() {

        let url = "http://example.com:5984".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        assert_eq!(Some(&[] as &[String]), transport.server_base_url.path());

        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        assert_eq!(Some(&[] as &[String]), transport.server_base_url.path());

        let url = "http://example.com:5984/foo".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        assert_eq!(Some(&["foo".to_string()] as &[String]),
                   transport.server_base_url.path());

        let url = "http://example.com:5984/foo/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        assert_eq!(Some(&["foo".to_string()] as &[String]),
                   transport.server_base_url.path());
    }
}
