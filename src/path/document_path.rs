use Error;
use error::PathParseErrorKind;
use super::{PathExtraction, PathExtractor};
use super::*;

impl<'a> DocumentPathRef<'a> {
    pub fn database_name(&self) -> DatabaseNameRef<'a> {
        self.db_name
    }

    pub fn document_id(&self) -> DocumentIdRef<'a> {
        self.doc_id
    }
}

impl DocumentPath {
    #[doc(hidden)]
    pub fn new(db_name: DatabaseName, doc_id: DocumentId) -> Self {
        DocumentPath {
            db_name: db_name,
            doc_id: doc_id,
        }
    }

    #[doc(hidden)]
    pub fn parse(s: &'static str) -> Result<Self, Error> {
        Ok(try!(s.into_document_path()).into())
    }

    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.doc_id
    }

    pub fn as_ref(&self) -> DocumentPathRef {
        DocumentPathRef {
            db_name: self.db_name.as_ref(),
            doc_id: self.doc_id.as_ref(),
        }
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DocumentPathIter {
        self.as_ref().into_iter()
    }
}

impl<'a> From<&'a DocumentPath> for DocumentPathRef<'a> {
    fn from(doc_path: &'a DocumentPath) -> Self {
        DocumentPathRef {
            db_name: doc_path.db_name.as_ref(),
            doc_id: doc_path.doc_id.as_ref(),
        }
    }
}

impl<'a> From<DocumentPathRef<'a>> for DocumentPath {
    fn from(doc_path: DocumentPathRef<'a>) -> Self {
        DocumentPath {
            db_name: doc_path.db_name.into(),
            doc_id: doc_path.doc_id.into(),
        }
    }
}

#[doc(hidden)]
impl<'a> IntoIterator for DocumentPathRef<'a> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = DocumentPathIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DocumentPathIter::DatabaseName(self)
    }
}

pub enum DocumentPathIter<'a> {
    DatabaseName(DocumentPathRef<'a>),
    DocumentPrefix(DocumentPathRef<'a>),
    DocumentName(DocumentPathRef<'a>),
    Done,
}

impl<'a> Iterator for DocumentPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self {
            &mut DocumentPathIter::DatabaseName(path) => {
                (path.db_name.inner,
                 match path.doc_id.prefix() {
                    None => DocumentPathIter::DocumentName(path),
                    Some(..) => DocumentPathIter::DocumentPrefix(path),
                })
            }
            &mut DocumentPathIter::DocumentPrefix(path) => {
                (path.doc_id.prefix().unwrap(),
                 DocumentPathIter::DocumentName(path))
            }
            &mut DocumentPathIter::DocumentName(path) => {
                (path.doc_id.name_as_str(), DocumentPathIter::Done)
            }
            &mut DocumentPathIter::Done => {
                return None;
            }
        };

        *self = next;
        Some(item)
    }
}

impl<'a> IntoDocumentPath<'a> for &'static str {
    fn into_document_path(self) -> Result<DocumentPathRef<'a>, Error> {

        let mut path_extractor = PathExtractor::new(self);
        let db_name = try!(path_extractor.extract_nonfinal());

        let doc_id = match try!(path_extractor.extract_any()) {
            PathExtraction::Final(segment) => DocumentIdRef::Normal(segment.into()),
            PathExtraction::Nonfinal(path_extractor, "_design") => {
                DocumentIdRef::Design(try!(path_extractor.extract_final()).into())
            }
            PathExtraction::Nonfinal(path_extractor, "_local") => {
                DocumentIdRef::Local(try!(path_extractor.extract_final()).into())
            }
            _ => {
                return Err(Error::PathParse(PathParseErrorKind::TooManySegments));
            }
        };

        Ok(DocumentPathRef {
            db_name: db_name.into(),
            doc_id: doc_id,
        })
    }
}

impl<'a> IntoDocumentPath<'a> for DocumentPathRef<'a> {
    fn into_document_path(self) -> Result<DocumentPathRef<'a>, Error> {
        Ok(self)
    }
}

impl<'a> IntoDocumentPath<'a> for &'a DocumentPath {
    fn into_document_path(self) -> Result<DocumentPathRef<'a>, Error> {
        Ok(self.into())
    }
}

impl<'a, T, U> IntoDocumentPath<'a> for (T, U)
    where T: IntoDatabasePath<'a>,
          U: Into<DocumentIdRef<'a>>
{
    fn into_document_path(self) -> Result<DocumentPathRef<'a>, Error> {
        Ok(DocumentPathRef {
            db_name: try!(self.0.into_database_path()).db_name,
            doc_id: self.1.into(),
        })
    }
}

#[cfg(test)]
mod tests {

    use Error;
    use error::PathParseErrorKind;
    use super::super::*;

    #[test]
    fn into_iter_normal() {
        let doc_path = "/foo/bar".into_document_path().unwrap();
        let expected = vec!["foo", "bar"];
        let got = doc_path.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_iter_design() {
        let doc_path = "/foo/_design/bar".into_document_path().unwrap();
        let expected = vec!["foo", "_design", "bar"];
        let got = doc_path.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_iter_local() {
        let doc_path = "/foo/_local/bar".into_document_path().unwrap();
        let expected = vec!["foo", "_local", "bar"];
        let got = doc_path.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_ok_normal() {
        let expected = DocumentPathRef {
            db_name: "foo".into(),
            doc_id: "bar".into(),
        };
        let got = "/foo/bar".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_ok_design() {
        let expected = DocumentPathRef {
            db_name: "foo".into(),
            doc_id: "_design/bar".into(),
        };
        let got = "/foo/_design/bar".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_ok_local() {
        let expected = DocumentPathRef {
            db_name: "foo".into(),
            doc_id: "_local/bar".into(),
        };
        let got = "/foo/_local/bar".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_nok_with_unexpected_prefix() {
        match "/foo/_bad_prefix/bar".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn from_tuple_of_static_str_refs() {
        let expected = DocumentPathRef {
            db_name: "foo".into(),
            doc_id: "_design/bar".into(),
        };
        let got = ("/foo", "_design/bar").into_document_path().unwrap();
        assert_eq!(expected, got);
    }
}
