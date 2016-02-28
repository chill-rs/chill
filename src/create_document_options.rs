use DocumentId;

#[derive(Debug, Default)]
pub struct CreateDocumentOptions {
    doc_id: Option<DocumentId>,
}

impl CreateDocumentOptions {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_document_id<D: Into<DocumentId>>(mut self, doc_id: D) -> Self {
        self.doc_id = Some(doc_id.into());
        self
    }

    pub fn document_id(&self) -> &Option<DocumentId> {
        &self.doc_id
    }
}
