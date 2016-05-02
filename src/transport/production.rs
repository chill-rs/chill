use Error;
use error::TransportErrorKind;
use hyper;
use serde;
use serde_json;
use super::{Action, RequestAccept, RequestBody, RequestOptions, Response, StatusCode, Transport};
use url;

fn make_url_path<'a, P: Iterator<Item = &'a str>>(base: &str, path: P) -> String {

    let without_trailing_slash: String = {
        let mut p = base;
        if p.ends_with('/') {
            p = &p[0..p.len() - 1];
        }
        p.into()
    };

    path.into_iter().fold(without_trailing_slash, {
        |mut left, right| {
            use url::percent_encoding::*;
            left.push('/');
            for encoded in utf8_percent_encode(right, PATH_SEGMENT_ENCODE_SET) {
                left.push_str(encoded);
            }
            left
        }
    })
}

#[cfg(test)]
mod make_url_path_tests {

    #[test]
    fn base_is_root() {
        use super::make_url_path;
        let expected = "/foo/bar";
        let got = make_url_path("/", vec!["foo", "bar"].into_iter());
        assert_eq!(expected, got);
    }

    #[test]
    fn base_in_nonroot_without_trailing_slash() {
        use super::make_url_path;
        let expected = "/foo/bar/qux";
        let got = make_url_path("/foo", vec!["bar", "qux"].into_iter());
        assert_eq!(expected, got);
    }

    #[test]
    fn base_in_nonroot_with_trailing_slash() {
        use super::make_url_path;
        let expected = "/foo/bar/qux";
        let got = make_url_path("/foo/", vec!["bar", "qux"].into_iter());
        assert_eq!(expected, got);
    }

    #[test]
    fn extra_contains_percent_char() {
        use super::make_url_path;
        let expected = "/foo%25bar";
        let got = make_url_path("/", vec!["foo%bar"].into_iter());
        assert_eq!(expected, got);
    }

    #[test]
    fn extra_contains_slash_char() {
        use super::make_url_path;
        let expected = "/foo%2Fbar";
        let got = make_url_path("/", vec!["foo/bar"].into_iter());
        assert_eq!(expected, got);
    }
}

#[derive(Debug)]
pub struct HyperTransport {
    server_base_url: url::Url,
    hyper_client: hyper::Client,
}

impl HyperTransport {
    pub fn new(server_base_url: url::Url) -> Result<Self, Error> {
        Ok(HyperTransport {
            server_base_url: server_base_url,
            hyper_client: hyper::Client::new(),
        })
    }

    pub fn exec_sync<A: Action<Self>>(&self, mut action: A) -> Result<A::Output, Error> {

        let (request, state) = try!(action.make_request());

        let response = {
            // FIXME: Remove `as_str` method call when Hyper upgrades to Url
            // v1.x.
            let b = self.hyper_client
                        .request(request.method, request.url.as_str())
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

        let full_path = make_url_path(url.path(), path.into_iter());
        url.set_path(&full_path);

        fn bool_to_string(x: bool) -> &'static str {
            if x {
                "true"
            } else {
                "false"
            }
        }

        {
            let mut query = url.query_pairs_mut();

            if let Some(yes_or_no) = options.attachments_query {
                query.append_pair("attachments", bool_to_string(yes_or_no));
            }

            if let Some(yes_or_no) = options.descending_query {
                query.append_pair("descending", bool_to_string(yes_or_no));
            }

            if let Some(ref key_value) = options.end_key_query {
                query.append_pair("endkey", key_value);
            }

            if let Some(yes_or_no) = options.inclusive_end_query {
                query.append_pair("inclusive_end", bool_to_string(yes_or_no));
            }

            if let Some(limit) = options.limit {
                query.append_pair("limit", &limit.to_string());
            }

            if let Some(yes_or_no) = options.reduce_query {
                query.append_pair("reduce", bool_to_string(yes_or_no));
            }

            if let Some(revision) = options.revision_query {
                query.append_pair("rev", &revision.to_string());
            }

            if let Some(ref key_value) = options.start_key_query {
                query.append_pair("startkey", key_value);
            }
        }

        if let Some("") = url.query() {
            url.set_query(None);
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
                         method: hyper::method::Method,
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
    method: hyper::method::Method,
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
    use super::*;
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
