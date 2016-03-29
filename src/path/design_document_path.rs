use Error;
use error::PathParseErrorKind;
use super::PathExtractor;
use super::*;

impl<'a> DesignDocumentPathRef<'a> {
    pub fn database_name(&self) -> DatabaseNameRef<'a> {
        self.db_name
    }

    pub fn design_document_name(&self) -> DesignDocumentNameRef<'a> {
        self.ddoc_name
    }
}

impl DesignDocumentPath {
    #[doc(hidden)]
    pub fn new(db_name: DatabaseName, ddoc_name: DesignDocumentName) -> Self {
        DesignDocumentPath {
            db_name: db_name,
            ddoc_name: ddoc_name,
        }
    }

    #[doc(hidden)]
    pub fn parse(s: &'static str) -> Result<Self, Error> {
        Ok(try!(s.into_design_document_path()).into())
    }

    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn design_document_name(&self) -> &DesignDocumentName {
        &self.ddoc_name
    }

    pub fn as_ref(&self) -> DesignDocumentPathRef {
        DesignDocumentPathRef {
            db_name: self.db_name.as_ref(),
            ddoc_name: self.ddoc_name.as_ref(),
        }
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DesignDocumentPathIter {
        self.as_ref().into_iter()
    }
}

impl<'a> From<&'a DesignDocumentPath> for DesignDocumentPathRef<'a> {
    fn from(ddoc_path: &'a DesignDocumentPath) -> Self {
        ddoc_path.as_ref()
    }
}

impl<'a, T, U> From<(T, U)> for DesignDocumentPathRef<'a>
    where T: Into<DatabasePathRef<'a>>,
          U: Into<DesignDocumentNameRef<'a>>
{
    fn from(parts: (T, U)) -> Self {
        DesignDocumentPathRef {
            db_name: parts.0.into().db_name,
            ddoc_name: parts.1.into(),
        }
    }
}

impl<'a> From<DesignDocumentPathRef<'a>> for DesignDocumentPath {
    fn from(ddoc_path: DesignDocumentPathRef<'a>) -> Self {
        DesignDocumentPath {
            db_name: ddoc_path.db_name.into(),
            ddoc_name: ddoc_path.ddoc_name.into(),
        }
    }
}

impl<T, U> From<(T, U)> for DesignDocumentPath
    where T: Into<DatabasePath>,
          U: Into<DesignDocumentName>
{
    fn from(parts: (T, U)) -> Self {
        DesignDocumentPath {
            db_name: parts.0.into().db_name,
            ddoc_name: parts.1.into(),
        }
    }
}

#[doc(hidden)]
impl<'a> IntoIterator for DesignDocumentPathRef<'a> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = DesignDocumentPathIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DesignDocumentPathIter::DatabaseName(self)
    }
}

pub enum DesignDocumentPathIter<'a> {
    DatabaseName(DesignDocumentPathRef<'a>),
    DocumentPrefix(DesignDocumentPathRef<'a>),
    DocumentName(DesignDocumentPathRef<'a>),
    Done,
}

impl<'a> Iterator for DesignDocumentPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self {
            &mut DesignDocumentPathIter::DatabaseName(path) => {
                (path.db_name.inner,
                 DesignDocumentPathIter::DocumentPrefix(path))
            }
            &mut DesignDocumentPathIter::DocumentPrefix(path) => {
                (DocumentId::design_prefix(),
                 DesignDocumentPathIter::DocumentName(path))
            }
            &mut DesignDocumentPathIter::DocumentName(path) => {
                (path.ddoc_name.inner, DesignDocumentPathIter::Done)
            }
            &mut DesignDocumentPathIter::Done => {
                return None;
            }
        };

        *self = next;
        Some(item)
    }
}

impl<'a> IntoDesignDocumentPath<'a> for &'static str {
    fn into_design_document_path(self) -> Result<DesignDocumentPathRef<'a>, Error> {

        let mut path_extractor = PathExtractor::new(self);
        let db_name = try!(path_extractor.extract_nonfinal());

        if DocumentId::design_prefix() != try!(path_extractor.extract_nonfinal()) {
            return Err(Error::PathParse(PathParseErrorKind::BadDesignPrefix));
        }

        let ddoc_name = try!(path_extractor.extract_final());

        Ok(DesignDocumentPathRef {
            db_name: db_name.into(),
            ddoc_name: ddoc_name.into(),
        })
    }
}

impl<'a> IntoDesignDocumentPath<'a> for DesignDocumentPathRef<'a> {
    fn into_design_document_path(self) -> Result<DesignDocumentPathRef<'a>, Error> {
        Ok(self)
    }
}

impl<'a> IntoDesignDocumentPath<'a> for &'a DesignDocumentPath {
    fn into_design_document_path(self) -> Result<DesignDocumentPathRef<'a>, Error> {
        Ok(self.into())
    }
}

impl<'a, T, U> IntoDesignDocumentPath<'a> for (T, U)
    where T: IntoDatabasePath<'a>,
          U: Into<DesignDocumentNameRef<'a>>
{
    fn into_design_document_path(self) -> Result<DesignDocumentPathRef<'a>, Error> {
        Ok(DesignDocumentPathRef {
            db_name: try!(self.0.into_database_path()).db_name,
            ddoc_name: self.1.into(),
        })
    }
}

#[cfg(test)]
mod tests {

    use Error;
    use error::PathParseErrorKind;
    use super::super::*;

    #[test]
    fn into_iter() {
        let ddoc_path = "/foo/_design/bar".into_design_document_path().unwrap();
        let expected = vec!["foo", "_design", "bar"];
        let got = ddoc_path.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_ok() {
        let expected = DesignDocumentPathRef {
            db_name: "foo".into(),
            ddoc_name: "bar".into(),
        };
        let got = "/foo/_design/bar".into_design_document_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_nok_bad_design_prefix() {
        match "/foo/invalid/bar".into_design_document_path() {
            Err(Error::PathParse(PathParseErrorKind::BadDesignPrefix)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn from_tuple_of_static_str_refs() {
        let expected = DesignDocumentPathRef {
            db_name: "foo".into(),
            ddoc_name: "bar".into(),
        };
        let got = ("/foo", "bar").into_design_document_path().unwrap();
        assert_eq!(expected, got);
    }
}
