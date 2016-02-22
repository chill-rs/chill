use action;
use DatabaseName;
use serde;
use std;
use transport::Transport;

#[derive(Debug)]
pub struct Database {
    transport: std::sync::Arc<Transport>,
    db_name: DatabaseName,
}

impl Database {
    #[doc(hidden)]
    pub fn new(transport: std::sync::Arc<Transport>, db_name: DatabaseName) -> Self {
        Database {
            db_name: db_name,
            transport: transport,
        }
    }

    pub fn create_document<'a, C>(&'a self, content: &'a C) -> action::CreateDocument<'a, C>
        where C: serde::Serialize
    {
        action::CreateDocument::new(&self.transport, self.db_name.clone(), content)
    }
}
