use DocumentId;

#[derive(Debug, Default, PartialEq)]
pub struct CreateDatabaseOptions;

#[derive(Debug, Default, PartialEq)]
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

#[derive(Debug, Default, PartialEq)]
pub struct DeleteDocumentOptions;

#[derive(Debug, Default, PartialEq)]
pub struct ReadDocumentOptions;

#[derive(Debug, Default, PartialEq)]
pub struct UpdateDocumentOptions;

#[cfg(test)]
mod tests {

    use DocumentId;
    use super::CreateDocumentOptions;

    #[test]
    fn create_document_with_document_id() {
        let expected = CreateDocumentOptions { doc_id: Some(DocumentId::from("document_id")) };
        let got = CreateDocumentOptions::new().with_document_id("document_id");
        assert_eq!(expected, got);
    }
}
