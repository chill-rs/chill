use AttachmentName;
use AttachmentNameRef;
use AttachmentPath;
use AttachmentPathRef;
use DatabaseName;
use DatabaseNameRef;
use DatabasePath;
use DatabasePathRef;
use DocumentId;
use DocumentIdRef;
use DocumentPath;
use DocumentPathRef;
use Error;
use IntoAttachmentPath;
use IntoDatabasePath;
use IntoDocumentPath;
use super::PathExtractor;

impl<'a> AttachmentPathRef<'a> {
    pub fn database_name(&self) -> DatabaseNameRef<'a> {
        self.db_name
    }

    pub fn document_id(&self) -> DocumentIdRef<'a> {
        self.doc_id
    }

    pub fn attachment_name(&self) -> AttachmentNameRef<'a> {
        self.att_name
    }
}

impl AttachmentPath {
    #[doc(hidden)]
    pub fn new(db_name: DatabaseName, doc_id: DocumentId, att_name: AttachmentName) -> Self {
        AttachmentPath {
            db_name: db_name,
            doc_id: doc_id,
            att_name: att_name,
        }
    }

    #[doc(hidden)]
    pub fn parse(s: &'static str) -> Result<Self, Error> {
        Ok(try!(s.into_attachment_path()).into())
    }

    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.doc_id
    }

    pub fn attachment_name(&self) -> &AttachmentName {
        &self.att_name
    }

    pub fn as_ref(&self) -> AttachmentPathRef {
        AttachmentPathRef {
            db_name: self.db_name.as_ref(),
            doc_id: self.doc_id.as_ref(),
            att_name: self.att_name.as_ref(),
        }
    }

    #[doc(hidden)]
    pub fn iter(&self) -> AttachmentPathIter {
        self.as_ref().into_iter()
    }
}

impl<'a> From<&'a AttachmentPath> for AttachmentPathRef<'a> {
    fn from(att_path: &'a AttachmentPath) -> Self {
        att_path.as_ref()
    }
}

impl<'a, T, U> From<(T, U)> for AttachmentPathRef<'a>
    where T: Into<DocumentPathRef<'a>>,
          U: Into<AttachmentNameRef<'a>>
{
    fn from(parts: (T, U)) -> Self {
        let doc_path = parts.0.into();
        AttachmentPathRef {
            db_name: doc_path.db_name,
            doc_id: doc_path.doc_id,
            att_name: parts.1.into(),
        }
    }
}

impl<'a, T, U, V> From<(T, U, V)> for AttachmentPathRef<'a>
    where T: Into<DatabasePathRef<'a>>,
          U: Into<DocumentIdRef<'a>>,
          V: Into<AttachmentNameRef<'a>>
{
    fn from(parts: (T, U, V)) -> Self {
        AttachmentPathRef {
            db_name: parts.0.into().db_name,
            doc_id: parts.1.into(),
            att_name: parts.2.into(),
        }
    }
}

impl<'a> From<AttachmentPathRef<'a>> for AttachmentPath {
    fn from(att_path: AttachmentPathRef<'a>) -> Self {
        AttachmentPath {
            db_name: att_path.db_name.into(),
            doc_id: att_path.doc_id.into(),
            att_name: att_path.att_name.into(),
        }
    }
}

impl<'a, T, U> From<(T, U)> for AttachmentPath
    where T: Into<DocumentPath>,
          U: Into<AttachmentName>
{
    fn from(parts: (T, U)) -> Self {
        let doc_path = parts.0.into();
        AttachmentPath {
            db_name: doc_path.db_name,
            doc_id: doc_path.doc_id,
            att_name: parts.1.into(),
        }
    }
}

impl<T, U, V> From<(T, U, V)> for AttachmentPath
    where T: Into<DatabasePath>,
          U: Into<DocumentId>,
          V: Into<AttachmentName>
{
    fn from(parts: (T, U, V)) -> Self {
        AttachmentPath {
            db_name: parts.0.into().db_name,
            doc_id: parts.1.into(),
            att_name: parts.2.into(),
        }
    }
}

#[doc(hidden)]
impl<'a> IntoIterator for AttachmentPathRef<'a> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = AttachmentPathIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        AttachmentPathIter::DatabaseName(self)
    }
}

pub enum AttachmentPathIter<'a> {
    DatabaseName(AttachmentPathRef<'a>),
    DocumentPrefix(AttachmentPathRef<'a>),
    DocumentName(AttachmentPathRef<'a>),
    AttachmentName(AttachmentPathRef<'a>),
    Done,
}

impl<'a> Iterator for AttachmentPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self {
            &mut AttachmentPathIter::DatabaseName(path) => {
                (path.db_name.inner,
                 match path.doc_id.prefix() {
                    None => AttachmentPathIter::DocumentName(path),
                    Some(..) => AttachmentPathIter::DocumentPrefix(path),
                })
            }
            &mut AttachmentPathIter::DocumentPrefix(path) => {
                (path.doc_id.prefix().unwrap(), AttachmentPathIter::DocumentName(path))
            }
            &mut AttachmentPathIter::DocumentName(path) => {
                (path.doc_id.name_as_str(), AttachmentPathIter::AttachmentName(path))
            }
            &mut AttachmentPathIter::AttachmentName(path) => {
                (path.att_name.inner, AttachmentPathIter::Done)
            }
            &mut AttachmentPathIter::Done => {
                return None;
            }
        };

        *self = next;
        Some(item)
    }
}

impl<'a> IntoAttachmentPath<'a> for &'static str {
    fn into_attachment_path(self) -> Result<AttachmentPathRef<'a>, Error> {

        let mut path_extractor = PathExtractor::new(self);
        let db_name = try!(path_extractor.extract_nonfinal());
        let (path_extractor, doc_id) = try!(path_extractor.extract_document_id_nonfinal());
        let att_name = try!(path_extractor.extract_final());

        Ok(AttachmentPathRef {
            db_name: db_name.into(),
            doc_id: doc_id,
            att_name: att_name.into(),
        })
    }
}

impl<'a> IntoAttachmentPath<'a> for AttachmentPathRef<'a> {
    fn into_attachment_path(self) -> Result<AttachmentPathRef<'a>, Error> {
        Ok(self)
    }
}

impl<'a> IntoAttachmentPath<'a> for &'a AttachmentPath {
    fn into_attachment_path(self) -> Result<AttachmentPathRef<'a>, Error> {
        Ok(self.into())
    }
}

impl<'a, T, U, V> IntoAttachmentPath<'a> for (T, U, V)
    where T: IntoDatabasePath<'a>,
          U: Into<DocumentIdRef<'a>>,
          V: Into<AttachmentNameRef<'a>>
{
    fn into_attachment_path(self) -> Result<AttachmentPathRef<'a>, Error> {
        Ok(AttachmentPathRef {
            db_name: try!(self.0.into_database_path()).db_name,
            doc_id: self.1.into(),
            att_name: self.2.into(),
        })
    }
}

impl<'a, T, U> IntoAttachmentPath<'a> for (T, U)
    where T: IntoDocumentPath<'a>,
          U: Into<AttachmentNameRef<'a>>
{
    fn into_attachment_path(self) -> Result<AttachmentPathRef<'a>, Error> {
        let doc_path = try!(self.0.into_document_path());
        Ok(AttachmentPathRef {
            db_name: doc_path.database_name(),
            doc_id: doc_path.document_id(),
            att_name: self.1.into(),
        })
    }
}

#[cfg(test)]
mod tests {

    use AttachmentPathRef;
    use IntoAttachmentPath;

    #[test]
    fn into_iter_normal() {
        let att_path = "/foo/bar/qux".into_attachment_path().unwrap();
        let expected = vec!["foo", "bar", "qux"];
        let got = att_path.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_iter_design() {
        let att_path = "/foo/_design/bar/qux".into_attachment_path().unwrap();
        let expected = vec!["foo", "_design", "bar", "qux"];
        let got = att_path.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_iter_local() {
        let att_path = "/foo/_local/bar/qux".into_attachment_path().unwrap();
        let expected = vec!["foo", "_local", "bar", "qux"];
        let got = att_path.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_ok_normal() {
        let expected = AttachmentPathRef {
            db_name: "foo".into(),
            doc_id: "bar".into(),
            att_name: "qux".into(),
        };
        let got = "/foo/bar/qux".into_attachment_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_ok_design() {
        let expected = AttachmentPathRef {
            db_name: "foo".into(),
            doc_id: "_design/bar".into(),
            att_name: "qux".into(),
        };
        let got = "/foo/_design/bar/qux".into_attachment_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_ok_local() {
        let expected = AttachmentPathRef {
            db_name: "foo".into(),
            doc_id: "_local/bar".into(),
            att_name: "qux".into(),
        };
        let got = "/foo/_local/bar/qux".into_attachment_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_tuple_2_of_static_str_refs() {
        let expected = AttachmentPathRef {
            db_name: "foo".into(),
            doc_id: "bar".into(),
            att_name: "qux".into(),
        };
        let got = ("/foo/bar", "qux").into_attachment_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_tuple_3_of_static_str_refs() {
        let expected = AttachmentPathRef {
            db_name: "foo".into(),
            doc_id: "bar".into(),
            att_name: "qux".into(),
        };
        let got = ("/foo", "bar", "qux").into_attachment_path().unwrap();
        assert_eq!(expected, got);
    }
}
