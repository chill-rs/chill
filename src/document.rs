use DatabaseName;
use DocumentId;
use Error;
use Revision;
use serde;
use serde_json;
use serializable_document::SerializableDocument;
use std;
use transport::{HyperTransport, Transport};

mod state {

    use Attachment;
    use DatabaseName;
    use DocumentId;
    use Revision;
    use std;
    use transport::Transport;

    // Base contains meta-information for all documents, including deleted
    // documents.
    #[derive(Debug)]
    pub struct Base<T: Transport> {
        pub transport: std::sync::Arc<T>,
        pub db_name: DatabaseName,
        pub doc_id: DocumentId,
        pub revision: Revision,
    }

    // Extra contains meta-information for non-deleted documents that doesn't
    // exist for deleted documents.
    #[derive(Debug, Default)]
    pub struct Extra {
        pub attachments: std::collections::HashMap<String, Attachment>,
    }
}

pub type Document = BasicDocument<HyperTransport>;

#[derive(Debug)]
pub enum BasicDocument<T: Transport> {
    #[doc(hidden)]
    Deleted {
        base: state::Base<T>,
    },

    #[doc(hidden)]
    Exists {
        base: state::Base<T>,
        extra: state::Extra,
        content: serde_json::Value,
    },
}

impl<T: Transport> BasicDocument<T> {
    #[doc(hidden)]
    pub fn from_serializable_document(transport: &std::sync::Arc<T>,
                                      db_name: &DatabaseName,
                                      doc: SerializableDocument)
                                      -> Self {

        let base = state::Base {
            transport: transport.clone(),
            db_name: db_name.clone(),
            doc_id: doc.id,
            revision: doc.revision,
        };

        match doc.deleted {
            true => BasicDocument::Deleted { base: base },
            false => {
                BasicDocument::Exists {
                    base: base,
                    extra: state::Extra { attachments: doc.attachments },
                    content: doc.content,
                }
            }

        }
    }

    pub fn database_name(&self) -> &DatabaseName {
        match self {
            &BasicDocument::Deleted { ref base, .. } => &base.db_name,
            &BasicDocument::Exists { ref base, .. } => &base.db_name,
        }
    }

    pub fn id(&self) -> &DocumentId {
        match self {
            &BasicDocument::Deleted { ref base, .. } => &base.doc_id,
            &BasicDocument::Exists { ref base, .. } => &base.doc_id,
        }
    }


    pub fn revision(&self) -> &Revision {
        match self {
            &BasicDocument::Deleted { ref base, .. } => &base.revision,
            &BasicDocument::Exists { ref base, .. } => &base.revision,
        }
    }

    pub fn get_content<C: serde::Deserialize>(&self) -> Result<C, Error> {
        match self {
            &BasicDocument::Deleted { .. } => Err(Error::DocumentIsDeleted),
            &BasicDocument::Exists { ref content, .. } => {
                serde_json::from_value(content.clone()).map_err(|e| Error::JsonDecode { cause: e })
            }
        }
    }

    pub fn set_content<C: serde::Serialize>(&mut self, new_content: &C) -> Result<(), Error> {
        match self {
            &mut BasicDocument::Deleted { .. } => Err(Error::DocumentIsDeleted),
            &mut BasicDocument::Exists { ref mut content, .. } => {
                *content = serde_json::to_value(new_content);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use DatabaseName;
    use DocumentId;
    use Error;
    use Revision;
    use serde_json;
    use std;
    use super::{BasicDocument, state};
    use transport::MockTransport;

    fn new_mock_base<N, I>(db_name: N, doc_id: I, revision: Revision) -> state::Base<MockTransport>
        where N: Into<DatabaseName>,
              I: Into<DocumentId>
    {
        state::Base {
            transport: std::sync::Arc::new(MockTransport::new()),
            db_name: db_name.into(),
            doc_id: doc_id.into(),
            revision: revision,
        }
    }

    #[test]
    fn get_content_ok() {

        let content = serde_json::builder::ObjectBuilder::new()
                          .insert("field_1", 42)
                          .insert("field_2", "foo")
                          .unwrap();

        let doc = BasicDocument::Exists {
            base: new_mock_base("database_name",
                                "document_id",
                                "1-1234567890abcdef1234567890abcdef".parse().unwrap()),
            extra: state::Extra { attachments: std::collections::HashMap::new() },
            content: content.clone(),
        };

        let expected = content;
        let got = doc.get_content().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn get_content_nok_document_is_deleted() {

        let doc = BasicDocument::Deleted {
            base: new_mock_base("database_name",
                                "document_id",
                                "1-1234567890abcdef1234567890abcdef".parse().unwrap()),
        };

        match doc.get_content::<serde_json::Value>() {
            Err(Error::DocumentIsDeleted) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn get_content_nok_decode_error() {

        let doc = BasicDocument::Exists {
            base: new_mock_base("database_name",
                                "document_id",
                                "1-1234567890abcdef1234567890abcdef".parse().unwrap()),
            extra: Default::default(),
            content: serde_json::builder::ObjectBuilder::new().unwrap(),
        };

        match doc.get_content::<i32>() {
            Err(Error::JsonDecode { .. }) => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
