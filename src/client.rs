use hyper;
use prelude_impl::*;
use serde;

/// Types that are convertible into a URL.
pub trait IntoUrl: hyper::client::IntoUrl {
}

impl<T: hyper::client::IntoUrl> IntoUrl for T {}

/// Manages all communication with a CouchDB server.
///
/// A `Client` communicates with the CouchDB server via **actions**. Each action
/// abstracts a single HTTP request-response pair, such as to create a database
/// (i.e., PUT `/db`) or read a document (i.e., GET `/db/doc`).
///
/// A `Client` communicates with exactly one CouchDB server, as specified by the
/// URI set when the `Client` is constructed.
///
pub struct Client {
    transport: HyperTransport,
}

impl Client {
    /// Constructs a client for the given server.
    pub fn new<U: IntoUrl>(server_url: U) -> Result<Self, Error> {
        let server_url = try!(server_url.into_url().map_err(|e| Error::UrlParse { cause: e }));
        let transport = try!(HyperTransport::new(server_url));
        Ok((Client { transport: transport }))
    }

    /// Builds an action to create a database.
    pub fn create_database<'a, P>(&'a self,
                                  db_path: P)
                                  -> Result<action::CreateDatabase<'a, HyperTransport>, Error>
        where P: IntoDatabasePath<'a>
    {
        action::CreateDatabase::new(&self.transport, db_path)
    }

    /// Builds an action to create a document.
    pub fn create_document<'a, C, P>
        (&'a self,
         db_path: P,
         content: &'a C)
         -> Result<action::CreateDocument<'a, HyperTransport, C>, Error>
        where C: serde::Serialize,
              P: IntoDatabasePath<'a>
    {
        action::CreateDocument::new(&self.transport, db_path, content)
    }

    /// Builds an action to read a document.
    pub fn read_document<'a, P>(&'a self,
                                doc_path: P)
                                -> Result<action::ReadDocument<'a, HyperTransport>, Error>
        where P: IntoDocumentPath<'a>
    {
        action::ReadDocument::new(&self.transport, doc_path)
    }

    /// Builds an action to update a document.
    pub fn update_document<'a>(&'a self,
                               doc: &'a Document)
                               -> Result<action::UpdateDocument<'a, HyperTransport>, Error> {
        action::UpdateDocument::new(&self.transport, doc)
    }

    /// Builds an action to delete a document.
    pub fn delete_document<'a, P>(&'a self,
                                  doc_path: P,
                                  revision: &'a Revision)
                                  -> Result<action::DeleteDocument<'a, HyperTransport>, Error>
        where P: IntoDocumentPath<'a>
    {
        action::DeleteDocument::new(&self.transport, doc_path, revision)
    }

    /// Builds an action to execute a view.
    pub fn execute_view<'a, K, V, P>
        (&'a self,
         view_path: P)
         -> Result<action::ExecuteView<'a, HyperTransport, K, V>, Error>
        where K: serde::Deserialize + serde::Serialize,
              P: IntoViewPath<'a>,
              V: serde::Deserialize
    {
        action::ExecuteView::new(&self.transport, view_path)
    }
}
