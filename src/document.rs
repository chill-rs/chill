// use DatabaseName;
// use DocumentId;
// use Error;
// use Revision;
// use serde;
// use serde_json;
// use std;
// use transport::Transport;
//
// mod state {
//
//     use DatabaseName;
//     use DocumentId;
//     use Revision;
//     use std;
//     use transport::Transport;
//
//     // Base contains meta-information for all documents, including deleted
//     // documents.
//     #[derive(Debug)]
//     pub struct Base {
//         pub transport: std::sync::Arc<Transport>,
//         pub db_name: DatabaseName,
//         pub doc_id: DocumentId,
//         pub revision: Revision,
//     }
//
//     // Extra contains meta-information for non-deleted documents that doesn't
//     // exist for deleted documents.
//     #[derive(Debug)]
//     pub struct Extra {
//         pub attachments: (), // FIXME: Implement attachment info.
//     }
// }
//
// #[derive(Debug)]
// pub struct DocumentMeta {
//     base: state::Base,
//     extra: state::Extra,
// }
//
// #[derive(Debug)]
// pub enum Document {
//     #[doc(hidden)]
//     Deleted {
//         base: state::Base,
//     },
//
//     #[doc(hidden)]
//     Exists {
//         base: state::Base,
//         extra: state::Extra,
//         content: serde_json::Value,
//     },
// }
//
// impl Document {
//     #[doc(hidden)]
//     pub fn new(transport: std::sync::Arc<Transport>,
//                db_name: DatabaseName,
//                doc_id: DocumentId,
//                revision: Revision,
//                content: serde_json::Value)
//                -> Self {
//         Document::Exists {
//             base: state::Base {
//                 transport: transport,
//                 db_name: db_name,
//                 doc_id: doc_id,
//                 revision: revision,
//             },
//             extra: state::Extra { attachments: () },
//             content: content,
//         }
//     }
//
//     pub fn into_content<C>(self) -> Result<(DocumentMeta, C), Error>
//         where C: serde::Deserialize
//     {
//         match self {
//             Document::Deleted { .. } => Err(Error::DocumentIsDeleted),
//             Document::Exists { base, extra, content } => {
//
//                 let content = try!(serde_json::from_value(content)
//                                        .map_err(|e| Error::JsonDecode { cause: e }));
//
//                 let meta = DocumentMeta {
//                     base: base,
//                     extra: extra,
//                 };
//
//                 Ok((meta, content))
//             }
//         }
//     }
//
//     pub fn from_content<C>(meta: DocumentMeta, content: C) -> Self
//         where C: serde::Serialize
//     {
//         Document::Exists {
//             base: meta.base,
//             extra: meta.extra,
//             content: serde_json::to_value(&content),
//         }
//     }
//
//     pub fn database_name(&self) -> &DatabaseName {
//         match self {
//             &Document::Deleted { ref base, .. } => &base.db_name,
//             &Document::Exists { ref base, .. } => &base.db_name,
//         }
//     }
//
//     pub fn id(&self) -> &DocumentId {
//         match self {
//             &Document::Deleted { ref base, .. } => &base.doc_id,
//             &Document::Exists { ref base, .. } => &base.doc_id,
//         }
//     }
//
//
//     pub fn revision(&self) -> &Revision {
//         match self {
//             &Document::Deleted { ref base, .. } => &base.revision,
//             &Document::Exists { ref base, .. } => &base.revision,
//         }
//     }
// }
