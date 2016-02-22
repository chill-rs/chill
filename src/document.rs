use DatabaseName;
use DocumentId;
use Revision;
use std;
use transport::Transport;

#[derive(Debug)]
pub struct Document {
    _transport: std::sync::Arc<Transport>,
    _db_name: DatabaseName,
    _id: DocumentId,
    _revision: Revision,
}

impl Document {
    #[doc(hidden)]
    pub fn new(transport: std::sync::Arc<Transport>,
               db_name: DatabaseName,
               doc_id: DocumentId,
               revision: Revision)
               -> Self {
        Document {
            _db_name: db_name,
            _id: doc_id,
            _revision: revision,
            _transport: transport,
        }
    }
}
