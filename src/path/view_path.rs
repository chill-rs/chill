use prelude_impl::*;
use super::PathExtractor;

fn view_prefix() -> &'static str {
    "_view"
}

impl<'a> ViewPathRef<'a> {
    pub fn database_name(&self) -> DatabaseNameRef<'a> {
        self.db_name
    }

    pub fn design_document_name(&self) -> DesignDocumentNameRef<'a> {
        self.ddoc_name
    }

    pub fn view_name(&self) -> ViewNameRef<'a> {
        self.view_name
    }
}

impl ViewPath {
    #[doc(hidden)]
    pub fn new(db_name: DatabaseName, ddoc_name: DesignDocumentName, view_name: ViewName) -> Self {
        ViewPath {
            db_name: db_name,
            ddoc_name: ddoc_name,
            view_name: view_name,
        }
    }

    #[doc(hidden)]
    pub fn parse(s: &'static str) -> Result<Self, Error> {
        Ok(try!(s.into_view_path()).into())
    }

    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn design_document_name(&self) -> &DesignDocumentName {
        &self.ddoc_name
    }

    pub fn view_name(&self) -> &ViewName {
        &self.view_name
    }

    pub fn as_ref(&self) -> ViewPathRef {
        ViewPathRef {
            db_name: self.db_name.as_ref(),
            ddoc_name: self.ddoc_name.as_ref(),
            view_name: self.view_name.as_ref(),
        }
    }

    #[doc(hidden)]
    pub fn iter(&self) -> ViewPathIter {
        self.as_ref().into_iter()
    }
}

impl<'a> From<&'a ViewPath> for ViewPathRef<'a> {
    fn from(view_path: &'a ViewPath) -> Self {
        view_path.as_ref()
    }
}

impl<'a, T, U> From<(T, U)> for ViewPathRef<'a>
    where T: Into<DesignDocumentPathRef<'a>>,
          U: Into<ViewNameRef<'a>>
{
    fn from(parts: (T, U)) -> Self {
        let ddoc_path = parts.0.into();
        ViewPathRef {
            db_name: ddoc_path.db_name,
            ddoc_name: ddoc_path.ddoc_name,
            view_name: parts.1.into(),
        }
    }
}

impl<'a, T, U, V> From<(T, U, V)> for ViewPathRef<'a>
    where T: Into<DatabasePathRef<'a>>,
          U: Into<DesignDocumentNameRef<'a>>,
          V: Into<ViewNameRef<'a>>
{
    fn from(parts: (T, U, V)) -> Self {
        ViewPathRef {
            db_name: parts.0.into().db_name,
            ddoc_name: parts.1.into(),
            view_name: parts.2.into(),
        }
    }
}

impl<'a> From<ViewPathRef<'a>> for ViewPath {
    fn from(view_path: ViewPathRef<'a>) -> Self {
        ViewPath {
            db_name: view_path.db_name.into(),
            ddoc_name: view_path.ddoc_name.into(),
            view_name: view_path.view_name.into(),
        }
    }
}

impl<'a, T, U> From<(T, U)> for ViewPath
    where T: Into<DesignDocumentPath>,
          U: Into<ViewName>
{
    fn from(parts: (T, U)) -> Self {
        let ddoc_path = parts.0.into();
        ViewPath {
            db_name: ddoc_path.db_name,
            ddoc_name: ddoc_path.ddoc_name,
            view_name: parts.1.into(),
        }
    }
}

impl<T, U, V> From<(T, U, V)> for ViewPath
    where T: Into<DatabasePath>,
          U: Into<DesignDocumentName>,
          V: Into<ViewName>
{
    fn from(parts: (T, U, V)) -> Self {
        ViewPath {
            db_name: parts.0.into().db_name,
            ddoc_name: parts.1.into(),
            view_name: parts.2.into(),
        }
    }
}

#[doc(hidden)]
impl<'a> IntoIterator for ViewPathRef<'a> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = ViewPathIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ViewPathIter::DatabaseName(self)
    }
}

pub enum ViewPathIter<'a> {
    DatabaseName(ViewPathRef<'a>),
    DocumentPrefix(ViewPathRef<'a>),
    DocumentName(ViewPathRef<'a>),
    ViewPrefix(ViewPathRef<'a>),
    ViewName(ViewPathRef<'a>),
    Done,
}

impl<'a> Iterator for ViewPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self {
            &mut ViewPathIter::DatabaseName(path) => {
                (path.db_name.inner, ViewPathIter::DocumentPrefix(path))
            }
            &mut ViewPathIter::DocumentPrefix(path) => {
                (DocumentId::design_prefix(),
                 ViewPathIter::DocumentName(path))
            }
            &mut ViewPathIter::DocumentName(path) => {
                (path.ddoc_name.inner, ViewPathIter::ViewPrefix(path))
            }
            &mut ViewPathIter::ViewPrefix(path) => (view_prefix(), ViewPathIter::ViewName(path)),
            &mut ViewPathIter::ViewName(path) => (path.view_name.inner, ViewPathIter::Done),
            &mut ViewPathIter::Done => {
                return None;
            }
        };

        *self = next;
        Some(item)
    }
}

impl<'a> IntoViewPath<'a> for &'static str {
    fn into_view_path(self) -> Result<ViewPathRef<'a>, Error> {

        let mut path_extractor = PathExtractor::new(self);
        let db_name = try!(path_extractor.extract_nonfinal());

        if DocumentId::design_prefix() != try!(path_extractor.extract_nonfinal()) {
            return Err(Error::PathParse(PathParseErrorKind::BadDesignPrefix));
        }

        let ddoc_name = try!(path_extractor.extract_nonfinal());

        if view_prefix() != try!(path_extractor.extract_nonfinal()) {
            return Err(Error::PathParse(PathParseErrorKind::BadViewPrefix));
        }

        let view_name = try!(path_extractor.extract_final());

        Ok(ViewPathRef {
            db_name: db_name.into(),
            ddoc_name: ddoc_name.into(),
            view_name: view_name.into(),
        })
    }
}

impl<'a> IntoViewPath<'a> for ViewPathRef<'a> {
    fn into_view_path(self) -> Result<ViewPathRef<'a>, Error> {
        Ok(self)
    }
}

impl<'a> IntoViewPath<'a> for &'a ViewPath {
    fn into_view_path(self) -> Result<ViewPathRef<'a>, Error> {
        Ok(self.into())
    }
}

impl<'a, T, U, V> IntoViewPath<'a> for (T, U, V)
    where T: IntoDatabasePath<'a>,
          U: Into<DesignDocumentNameRef<'a>>,
          V: Into<ViewNameRef<'a>>
{
    fn into_view_path(self) -> Result<ViewPathRef<'a>, Error> {
        Ok(ViewPathRef {
            db_name: try!(self.0.into_database_path()).db_name,
            ddoc_name: self.1.into(),
            view_name: self.2.into(),
        })
    }
}

impl<'a, T, U> IntoViewPath<'a> for (T, U)
    where T: IntoDesignDocumentPath<'a>,
          U: Into<ViewNameRef<'a>>
{
    fn into_view_path(self) -> Result<ViewPathRef<'a>, Error> {
        let ddoc_path = try!(self.0.into_design_document_path());
        Ok(ViewPathRef {
            db_name: ddoc_path.database_name(),
            ddoc_name: ddoc_path.design_document_name(),
            view_name: self.1.into(),
        })
    }
}

#[cfg(test)]
mod tests {

    use prelude_impl::*;

    #[test]
    fn into_iter() {
        let view_path = "/foo/_design/bar/_view/qux".into_view_path().unwrap();
        let expected = vec!["foo", "_design", "bar", "_view", "qux"];
        let got = view_path.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_ok() {
        let expected = ViewPathRef {
            db_name: "foo".into(),
            ddoc_name: "bar".into(),
            view_name: "qux".into(),
        };
        let got = "/foo/_design/bar/_view/qux".into_view_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_nok_bad_design_prefix() {
        match "/foo/invalid/bar/_view/qux".into_view_path() {
            Err(Error::PathParse(PathParseErrorKind::BadDesignPrefix)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn from_static_str_ref_nok_bad_view_prefix() {
        match "/foo/_design/bar/invalid/qux".into_view_path() {
            Err(Error::PathParse(PathParseErrorKind::BadViewPrefix)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn from_tuple_2_of_static_str_refs() {
        let expected = ViewPathRef {
            db_name: "foo".into(),
            ddoc_name: "bar".into(),
            view_name: "qux".into(),
        };
        let got = ("/foo/_design/bar", "qux").into_view_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_tuple_3_of_static_str_refs() {
        let expected = ViewPathRef {
            db_name: "foo".into(),
            ddoc_name: "bar".into(),
            view_name: "qux".into(),
        };
        let got = ("/foo", "bar", "qux").into_view_path().unwrap();
        assert_eq!(expected, got);
    }
}
