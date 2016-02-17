use action;
use DatabaseName;
use std;
use transport::Transport;

pub struct Database {
    transport: std::sync::Arc<Transport>,
    db_name: DatabaseName,
}

impl Database {
    pub fn new(transport: std::sync::Arc<Transport>, db_name: DatabaseName) -> Self {
        Database {
            db_name: db_name,
            transport: transport,
        }
    }

    pub fn create<'a>(&'a self) -> action::CreateDatabase<'a> {
        use std::ops::Deref;
        action::CreateDatabase::new(self.transport.deref(), self.db_name.clone())
    }
}
