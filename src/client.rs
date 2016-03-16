use action;
use Document;
use Error;
use hyper;
use IntoDatabasePath;
use IntoDocumentPath;
use Revision;
use serde;
use transport::{HyperTransport, Transport};

/// The `IntoUrl` trait applies to a type that is convertible into a URL.
pub trait IntoUrl: hyper::client::IntoUrl {
}

impl<T: hyper::client::IntoUrl> IntoUrl for T {}

pub type Client = BasicClient<HyperTransport>;

impl Client {
    pub fn new<U: IntoUrl>(server_url: U) -> Result<Self, Error> {
        let server_url = try!(server_url.into_url().map_err(|e| Error::UrlParse { cause: e }));
        let transport = try!(HyperTransport::new(server_url));
        Ok((BasicClient { transport: transport }))
    }
}

pub struct BasicClient<T: Transport> {
    transport: T,
}

impl<T: Transport> BasicClient<T> {
    pub fn create_database<'a, P>(&'a self, db_path: P) -> action::CreateDatabase<'a, P, T>
        where P: IntoDatabasePath<'a>
    {
        action::CreateDatabase::new(&self.transport, db_path)
    }

    pub fn create_document<'a, C, P>(&'a self,
                                     db_path: P,
                                     content: &'a C)
                                     -> action::CreateDocument<'a, C, P, T>
        where C: serde::Serialize,
              P: IntoDatabasePath<'a>
    {
        action::CreateDocument::new(&self.transport, db_path, content)
    }

    pub fn read_document<'a, P>(&'a self, doc_path: P) -> action::ReadDocument<'a, P, T>
        where P: IntoDocumentPath<'a>
    {
        action::ReadDocument::new(&self.transport, doc_path)
    }

    pub fn update_document<'a>(&'a self, doc: &'a Document) -> action::UpdateDocument<'a, T> {
        action::UpdateDocument::new(&self.transport, doc)
    }

    pub fn delete_document<'a, P>(&'a self,
                                  doc_path: P,
                                  revision: &'a Revision)
                                  -> action::DeleteDocument<'a, P, T>
        where P: IntoDocumentPath<'a>
    {
        action::DeleteDocument::new(&self.transport, doc_path, revision)
    }
}
