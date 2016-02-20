use DatabaseName;
use std;
use transport::Transport;

#[derive(Debug)]
pub struct Database {
    _transport: std::sync::Arc<Transport>,
    _db_name: DatabaseName,
}

impl Database {
    #[doc(hidden)]
    pub fn new(transport: std::sync::Arc<Transport>, db_name: DatabaseName) -> Self {
        Database {
            _db_name: db_name,
            _transport: transport,
        }
    }
}
