use Error;
use super::*;
use super::path_extract_final;
use super::path_extract_nonfinal;

impl<'a> DocumentPath<'a> {
    #[doc(hidden)]
    pub fn database_name(&self) -> &'a DatabaseName {
        &self.db_name
    }

    #[doc(hidden)]
    pub fn document_id(&self) -> &DocumentId<'a> {
        &self.doc_id
    }
}

#[doc(hidden)]
impl<'a> IntoIterator for DocumentPath<'a> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = DocumentPathIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DocumentPathIter {
            doc_path: self,
            state: DocumentPathIterState::DatabaseName,
        }
    }
}

impl DocumentPathBuf {
    #[doc(hidden)]
    pub fn new_from_parts(db_name: DatabaseNameBuf, doc_id: DocumentIdBuf) -> Self {
        DocumentPathBuf {
            db_name_buf: db_name,
            doc_id_buf: doc_id,
        }
    }

    #[doc(hidden)]
    pub fn parse(s: &'static str) -> Result<Self, Error> {
        let path = try!(s.into_document_path());
        Ok(DocumentPathBuf {
            db_name_buf: path.db_name.to_owned(),
            doc_id_buf: DocumentIdBuf::from(path.doc_id),
        })
    }

    #[doc(hidden)]
    pub fn as_document_path(&self) -> DocumentPath {
        DocumentPath {
            db_name: &self.db_name_buf,
            doc_id: self.doc_id_buf.as_document_id(),
        }
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DocumentPathIter {
        self.as_document_path().into_iter()
    }

    pub fn database_name(&self) -> &DatabaseNameBuf {
        &self.db_name_buf
    }

    pub fn document_id(&self) -> &DocumentIdBuf {
        &self.doc_id_buf
    }
}

#[doc(hidden)]
impl<'a> From<DocumentPath<'a>> for DocumentPathBuf {
    fn from(doc_path: DocumentPath<'a>) -> Self {
        DocumentPathBuf {
            db_name_buf: doc_path.db_name.to_owned(),
            doc_id_buf: doc_path.doc_id.into(),
        }
    }
}

pub struct DocumentPathIter<'a> {
    doc_path: DocumentPath<'a>,
    state: DocumentPathIterState,
}

enum DocumentPathIterState {
    DatabaseName,
    DocumentPrefix,
    DocumentName,
    Done,
}

impl<'a> Iterator for DocumentPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self.state {
            DocumentPathIterState::DatabaseName => {
                (&self.doc_path.db_name.inner,
                 if self.doc_path.doc_id.has_prefix() {
                    DocumentPathIterState::DocumentPrefix
                } else {
                    DocumentPathIterState::DocumentName
                })
            }
            DocumentPathIterState::DocumentPrefix => {
                (self.doc_path.doc_id.prefix_as_str().unwrap(),
                 DocumentPathIterState::DocumentName)
            }
            DocumentPathIterState::DocumentName => {
                (self.doc_path.doc_id.name_as_str(),
                 DocumentPathIterState::Done)
            }
            DocumentPathIterState::Done => {
                return None;
            }
        };

        self.state = next;
        Some(item)
    }
}

impl<'a> IntoDocumentPath<'a> for &'static str {
    fn into_document_path(self) -> Result<DocumentPath<'static>, Error> {

        let (db_name, remaining) = try!(path_extract_nonfinal(self));

        // The document id type is unusual in that it has a variable number of
        // segments.

        // FIXME: Because this function is infallible, _all_ strings comprise a
        // document id. This includes strings such as "_design" and "_local". Is
        // this a good idea? At the least, it should be documented.

        let design_prefix = "/_design";
        let local_prefix = "/_local";

        let doc_id = if remaining.starts_with(design_prefix) {
            let doc_name = try!(path_extract_final(&remaining[design_prefix.len()..]));
            DocumentId::Design(DesignDocumentName::new(doc_name))
        } else if remaining.starts_with(local_prefix) {
            let doc_name = try!(path_extract_final(&remaining[local_prefix.len()..]));
            DocumentId::Local(DocumentName::new(doc_name))
        } else {
            let doc_name = try!(path_extract_final(remaining));
            DocumentId::Normal(DocumentName::new(doc_name))
        };

        Ok(DocumentPath {
            db_name: DatabaseName::new(db_name),
            doc_id: doc_id,
        })
    }
}

impl<'a> IntoDocumentPath<'a> for DocumentPath<'a> {
    fn into_document_path(self) -> Result<DocumentPath<'a>, Error> {
        Ok(self)
    }
}

impl<'a> IntoDocumentPath<'a> for &'a DocumentPathBuf {
    fn into_document_path(self) -> Result<DocumentPath<'a>, Error> {
        Ok(DocumentPath {
            db_name: DatabaseName::new(&self.db_name_buf.inner),
            doc_id: self.doc_id_buf.as_document_id(),
        })
    }
}

impl<'a, T, U> IntoDocumentPath<'a> for (T, U)
    where T: IntoDatabasePath<'a>,
          U: Into<DocumentId<'a>>
{
    fn into_document_path(self) -> Result<DocumentPath<'a>, Error> {
        Ok(DocumentPath {
            db_name: try!(self.0.into_database_path()).database_name(),
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
    fn document_path_into_iter_normal() {
        let doc_path = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let got = doc_path.into_iter().collect::<Vec<_>>();
        assert_eq!(vec!["foo", "bar"], got);
    }

    #[test]
    fn document_path_into_iter_design() {
        let doc_path = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let got = doc_path.into_iter().collect::<Vec<_>>();
        assert_eq!(vec!["foo", "_design", "bar"], got);
    }

    #[test]
    fn document_path_into_iter_local() {
        let doc_path = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let got = doc_path.into_iter().collect::<Vec<_>>();
        assert_eq!(vec!["foo", "_local", "bar"], got);
    }

    #[test]
    fn document_path_buf_new_from_parts() {
        let expected = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got =
            DocumentPathBuf::new_from_parts(DatabaseNameBuf::from("foo"),
                                            DocumentIdBuf::Normal(DocumentNameBuf::from("bar")));
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_parse_normal() {
        let expected = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got = DocumentPathBuf::parse("/foo/bar").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_parse_design() {
        let expected = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Design(DesignDocumentNameBuf::from("bar")),
        };
        let got = DocumentPathBuf::parse("/foo/_design/bar").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_parse_local() {
        let expected = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Local(DocumentNameBuf::from("bar")),
        };
        let got = DocumentPathBuf::parse("/foo/_local/bar").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_as_document_path_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.as_document_path();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_as_document_path_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Design(DesignDocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.as_document_path();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_as_document_path_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Local(DocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.as_document_path();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_iter_normal() {
        let expected = vec!["foo", "bar"];
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_iter_design() {
        let expected = vec!["foo", "_design", "bar"];
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Design(DesignDocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_iter_local() {
        let expected = vec!["foo", "_local", "bar"];
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Local(DocumentNameBuf::from("bar")),
        };
        let got = doc_path_buf.iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn document_path_buf_from_document_path() {
        let expected = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got = DocumentPathBuf::from(DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        });
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_into_document_path_ok_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let got = "/foo/bar".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_into_document_path_ok_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let got = "/foo/_design/bar".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_into_document_path_ok_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let got = "/foo/_local/bar".into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_into_document_path_nok_no_leading_slash() {
        match "foo/bar".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_database_only() {
        match "/foo".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooFewSegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_empty_document_id() {
        match "/foo/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_empty_design_document_id() {
        match "/foo/_design/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_empty_local_document_id() {
        match "/foo/_local/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_too_many_segments_normal() {
        match "/foo/bar/qux".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_too_many_segments_design() {
        match "/foo/_design/bar/qux".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_too_many_segments_local() {
        match "/foo/_local/bar/qux".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_trailing_slash_normal() {
        match "/foo/bar/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_trailing_slash_design() {
        match "/foo/_design/bar/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_document_path_nok_trailing_slash_local() {
        match "/foo/_local/bar/".into_document_path() {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn into_document_path_with_document_id_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let got = ("/foo", DocumentId::Normal(DocumentName::new("bar")))
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_with_document_id_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let got = ("/foo", DocumentId::Design(DesignDocumentName::new("bar")))
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_with_document_id_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let got = ("/foo", DocumentId::Local(DocumentName::new("bar")))
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_with_document_id_buf_ref_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let doc_id_buf = DocumentIdBuf::Normal(DocumentNameBuf::from("bar"));
        let got = ("/foo", &doc_id_buf)
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_with_document_id_buf_ref_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let doc_id_buf = DocumentIdBuf::Design(DesignDocumentNameBuf::from("bar"));
        let got = ("/foo", &doc_id_buf)
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_with_document_id_buf_ref_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let doc_id_buf = DocumentIdBuf::Local(DocumentNameBuf::from("bar"));
        let got = ("/foo", &doc_id_buf)
                      .into_document_path()
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_buf_ref_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Normal(DocumentNameBuf::from("bar")),
        };
        let got = (&doc_path_buf).into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_buf_ref_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Design(DesignDocumentNameBuf::from("bar")),
        };
        let got = (&doc_path_buf).into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_buf_ref_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let doc_path_buf = DocumentPathBuf {
            db_name_buf: DatabaseNameBuf::from("foo"),
            doc_id_buf: DocumentIdBuf::Local(DocumentNameBuf::from("bar")),
        };
        let got = (&doc_path_buf).into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_normal() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Normal(DocumentName::new("bar")),
        };
        let got = expected.clone().into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_design() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Design(DesignDocumentName::new("bar")),
        };
        let got = expected.clone().into_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn into_document_path_from_document_path_local() {
        let expected = DocumentPath {
            db_name: DatabaseName::new("foo"),
            doc_id: DocumentId::Local(DocumentName::new("bar")),
        };
        let got = expected.clone().into_document_path().unwrap();
        assert_eq!(expected, got);
    }
}
