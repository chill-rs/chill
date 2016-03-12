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
        let uri = self.make_request_url(path, &options);

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

    fn make_request_url<'a, B>(&self, path: &[&str], options: &RequestOptions<'a, B>) -> url::Url
        where B: serde::Serialize
    {
        let mut url = self.server_base_url.clone();

        // The base URL may have an empty final path component, which will lead
        // to an empty path component (//) if we naively append path components
        // to the URL.
        if !path.is_empty() && url.path().unwrap().last().map_or(false, |x| x.is_empty()) {
            url.path_mut().unwrap().pop();
        }

        url.path_mut().unwrap().extend(path.iter().map(|x| {
            let x = x.replace("%", "%25")
                     .replace("/", "%2F");
            url::percent_encoding::utf8_percent_encode(&x,
                                                       url::percent_encoding::DEFAULT_ENCODE_SET)
        }));

        let query_pairs = {
            let mut pairs = std::collections::HashMap::new();
            if let Some(revision) = options.revision_query {
                pairs.insert("rev".to_string(), revision.to_string());
            }
            pairs
        };

        if !query_pairs.is_empty() {
            url.set_query_from_pairs(query_pairs);
        }

        url
    }
}

impl Transport for HyperTransport {
    type Response = HyperResponse;

    fn delete<'a, B>(&self,
                     path: &[&str],
                     options: RequestOptions<'a, B>)
                     -> Result<Self::Response, Error>
        where B: serde::Serialize
    {
        self.request(Method::Delete, path, options)
    }

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

    use Revision;
    use transport::RequestOptions;
    use super::HyperTransport;
    use url;

    #[test]
    fn make_request_url_empty() {
        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:5984/".parse().unwrap();
        let got = transport.make_request_url(&[], &RequestOptions::<()>::default());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_url_normal_path() {
        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:5984/db/docid".parse().unwrap();
        let got = transport.make_request_url(&["db", "docid"], &RequestOptions::<()>::default());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_url_base_url_has_nonempty_path() {
        let url = "http://example.com:5984/foo".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:5984/foo/db/docid".parse().unwrap();
        let got = transport.make_request_url(&["db", "docid"], &RequestOptions::<()>::default());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_url_base_url_has_nonempty_path_with_trailing_slash() {
        let url = "http://example.com:5984/foo/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:5984/foo/db/docid".parse().unwrap();
        let got = transport.make_request_url(&["db", "docid"], &RequestOptions::<()>::default());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_url_path_is_percent_encoded() {
        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:5984/foo%2F%3F%23%20%25bar"
                                     .parse()
                                     .unwrap();
        let got = transport.make_request_url(&["foo/?# %bar"], &RequestOptions::<()>::default());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_url_with_revision_query() {
        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:\
                                  5984/db/doc?rev=42-1234567890abcdef1234567890abcdef"
                                     .parse()
                                     .unwrap();

        let got = transport.make_request_url(&["db", "doc"], {
            &RequestOptions::<()>::new()
                 .with_revision_query(&Revision::parse("42-1234567890abcdef1234567890abcdef")
                                           .unwrap())
        });
        assert_eq!(expected, got);
    }
}
