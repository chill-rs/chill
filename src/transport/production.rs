use Error;
use error::TransportErrorKind;
use hyper;
use serde;
use serde_json;
use std;
use super::{Action, Method, RequestAccept, RequestBody, RequestOptions, Response, StatusCode,
            Transport};
use url;

#[derive(Debug)]
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

    pub fn exec_sync<A: Action<Self>>(&self, mut action: A) -> Result<A::Output, Error> {

        let (request, state) = try!(action.make_request());

        let response = {
            let b = self.hyper_client
                        .request(request.method, request.url)
                        .headers(request.headers);

            let b = if request.body.is_empty() {
                b
            } else {
                b.body(&request.body[..])
            };

            let response = try!(b.send().map_err(|e| {
                Error::Transport { kind: TransportErrorKind::Hyper(e) }
            }));

            HyperResponse { hyper_response: response }
        };

        A::take_response(response, state)
    }

    fn make_request_url<'a, B, P>(&self, path: P, options: &RequestOptions<'a, B>) -> url::Url
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        let mut url = self.server_base_url.clone();

        let path = path.into_iter().collect::<Vec<_>>();

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

    fn make_headers<'a, B>(&self, options: &RequestOptions<'a, B>) -> hyper::header::Headers
        where B: serde::Serialize
    {
        let mut headers = hyper::header::Headers::new();

        match options.accept {
            None => (),
            Some(RequestAccept::Json) => {
                let quality_items = vec![hyper::header::qitem(mime!(Application / Json))];
                headers.set(hyper::header::Accept(quality_items));
            }
        }

        match options.body {
            None => (),
            Some(RequestBody::Json(..)) => {
                headers.set(hyper::header::ContentType(mime!(Application / Json)));
            }
        }

        headers
    }
}

impl Transport for HyperTransport {
    type Request = HyperRequest;

    fn request<'a, B, P>(&self,
                         method: Method,
                         path: P,
                         options: RequestOptions<'a, B>)
                         -> Result<Self::Request, Error>
        where B: serde::Serialize,
              P: IntoIterator<Item = &'a str>
    {
        let url = self.make_request_url(path, &options);
        let headers = self.make_headers(&options);

        let body = match options.body {
            None => Vec::new(),
            Some(RequestBody::Json(body)) => {
                try!(serde_json::to_vec(body).map_err(|e| Error::JsonEncode { cause: e }))
            }
        };

        Ok(HyperRequest {
            method: method,
            url: url,
            headers: headers,
            body: body,
        })
    }
}

#[derive(Debug)]
pub struct HyperRequest {
    method: Method,
    url: url::Url,
    headers: hyper::header::Headers,
    body: Vec<u8>,
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

    use hyper;
    use Revision;
    use serde_json;
    use super::HyperTransport;
    use transport::RequestOptions;
    use url;

    #[test]
    fn make_request_url_empty() {
        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:5984/".parse().unwrap();
        let got = transport.make_request_url(vec![], &RequestOptions::<()>::default());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_url_normal_path() {
        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:5984/db/docid".parse().unwrap();
        let got = transport.make_request_url(vec!["db", "docid"], &RequestOptions::<()>::default());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_url_base_url_has_nonempty_path() {
        let url = "http://example.com:5984/foo".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:5984/foo/db/docid".parse().unwrap();
        let got = transport.make_request_url(vec!["db", "docid"], &RequestOptions::<()>::default());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_url_base_url_has_nonempty_path_with_trailing_slash() {
        let url = "http://example.com:5984/foo/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:5984/foo/db/docid".parse().unwrap();
        let got = transport.make_request_url(vec!["db", "docid"], &RequestOptions::<()>::default());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_url_path_is_percent_encoded() {
        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected: url::Url = "http://example.com:5984/foo%2F%3F%23%20%25bar"
                                     .parse()
                                     .unwrap();
        let got = transport.make_request_url(vec!["foo/?# %bar"], &RequestOptions::<()>::default());
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

        let got = transport.make_request_url(vec!["db", "doc"], {
            &RequestOptions::<()>::new()
                 .with_revision_query(&Revision::parse("42-1234567890abcdef1234567890abcdef")
                                           .unwrap())
        });
        assert_eq!(expected, got);
    }

    #[test]
    fn make_headers_default() {
        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected = hyper::header::Headers::new();
        let got = transport.make_headers(&RequestOptions::<()>::new());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_headers_with_accept_json() {
        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected = {
            let mut headers = hyper::header::Headers::new();
            let quality_items = vec![hyper::header::qitem(mime!(Application / Json))];
            headers.set(hyper::header::Accept(quality_items));
            headers
        };
        let got = transport.make_headers(&RequestOptions::<()>::new().with_accept_json());
        assert_eq!(expected, got);
    }

    #[test]
    fn make_headers_with_json_body() {
        let url = "http://example.com:5984/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        let expected = {
            let mut headers = hyper::header::Headers::new();
            headers.set(hyper::header::ContentType(mime!(Application / Json)));
            headers
        };
        let body = serde_json::builder::ObjectBuilder::new().unwrap();
        let got = transport.make_headers(&RequestOptions::<()>::new().with_json_body(&body));
        assert_eq!(expected, got);
    }
}
