use Error;
use error::TransportErrorKind;
use hyper;
use std;
use transport::{PrintableBytes, Request, Response, Transport};
use url;

pub struct HyperTransport {
    server_url: url::Url,
    hyper_client: hyper::Client,
}

impl HyperTransport {
    pub fn new(mut server_url: url::Url) -> Result<Self, Error> {

        if server_url.path_mut().is_none() {
            return Err(Error::UrlNotSchemeRelative);
        }

        // Sometimes the URL has an empty final path component, which will lead
        // to an empty path component (//) if we naively append path components
        // to the URL. Remove any empty final component now.

        {
            let mut path = server_url.path_mut().unwrap();
            if !path.is_empty() && path.last().unwrap().is_empty() {
                path.pop();
            }
        }

        Ok(HyperTransport {
            server_url: server_url,
            hyper_client: hyper::Client::new(),
        })
    }
}

// NOTE: This Debug implementation is a workaround for hyper::Client not
// implementing the Debug trait. This should be fixed in Hyper pull request #729
// (https://github.com/hyperium/hyper/pull/729).

impl std::fmt::Debug for HyperTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "HyperTransport {{ server_url: {:?} }}", self.server_url)
    }
}

impl Transport for HyperTransport {
    fn send(&self, request: Request) -> Result<Response, Error> {

        let url = {
            let mut url = self.server_url.clone();

            {
                let mut p = url.path_mut().unwrap();
                p.extend(request.path);
            }

            url.set_query_from_pairs(request.query);

            url
        };

        let PrintableBytes(body_bytes) = request.body;

        let hyper_response = try!(self.hyper_client
                                      .request(request.method, url)
                                      .headers(request.headers)
                                      .body(&body_bytes[..])
                                      .send()
                                      .map_err(|e| {
                                          Error::Transport { kind: TransportErrorKind::Hyper(e) }
                                      }));

        Response::from_hyper_response(hyper_response)
    }
}

#[cfg(test)]
mod tests {

    use Error;
    use super::HyperTransport;
    use url;

    #[test]
    fn new_transport_nok_url_not_scheme_relative() {
        let url: url::Url = "foo:bar".parse().unwrap();
        let got = HyperTransport::new(url);
        match got {
            Err(Error::UrlNotSchemeRelative) => (),
            Err(..) => panic!("Got unexpected error result {:?}", got),
            Ok(..) => panic!("Got unexpected OK result {:?}", got),
        }
    }

    #[test]
    fn new_transport_ok_url_with_trailing_slash() {
        let url: url::Url = "http://example.com:5984/foo/".parse().unwrap();
        let transport = HyperTransport::new(url).unwrap();
        assert_eq!(vec!["foo"], transport.server_url.path().unwrap());
    }
}
