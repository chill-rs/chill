use {Error, action, url};
use transport::NetTransport;

fn e_badly_formed_url(url_str: &str, e: url::ParseError) -> Error {
    Error::from((format!("The URL is badly formed (URL: {:?})", url_str), e))
}

/// `IntoUrl` is a trait for types that may be converted into a URL.
///
/// The `IntoUrl` trait is like the `Into` trait except that its conversion may
/// fail, such as when parsing a string containing an invalid URL.
///
pub trait IntoUrl {
    fn into_url(self) -> Result<url::Url, Error>;
}

impl IntoUrl for url::Url {
    fn into_url(self) -> Result<url::Url, Error> {
        Ok(self)
    }
}

impl<'a> IntoUrl for &'a str {
    fn into_url(self) -> Result<url::Url, Error> {
        let u = url::Url::parse(self).map_err(
            |e| e_badly_formed_url(self, e),
        )?;
        Ok(u)
    }
}

impl<'a> IntoUrl for &'a String {
    fn into_url(self) -> Result<url::Url, Error> {
        let u = url::Url::parse(self).map_err(
            |e| e_badly_formed_url(self, e),
        )?;
        Ok(u)
    }
}

#[derive(Debug)]
pub struct Client {
    transport: NetTransport,
}

impl Client {
    pub fn new<U: IntoUrl>(server_url: U) -> Result<Self, Error> {
        let server_url = server_url.into_url()?;
        let transport = NetTransport::new(server_url)?;
        Ok(Client { transport: transport })
    }

    pub fn create_database(&self, db_path: &str) -> action::CreateDatabase<NetTransport> {
        action::CreateDatabase::new(&self.transport, db_path)
    }
}
