use DatabaseName;
use DatabaseNameRef;
use DatabasePath;
use DatabasePathRef;
use DatabaseViewPathRef;
use DatabaseViewPath;
use ViewNameRef;
use ViewName;
use IntoDatabaseViewPath;
use Error;
use IntoDatabasePath;
use super::PathExtractor;

impl<'a> DatabaseViewPathRef<'a> {
    pub fn database_name(&self) -> DatabaseNameRef<'a> {
        self.db_name
    }

    pub fn view_name(&self) -> ViewNameRef<'a> {
        self.view_name
    }
}

impl DatabaseViewPath {
    #[doc(hidden)]
    pub fn new(db_name: DatabaseName, view_name: ViewName) -> Self {
        DatabaseViewPath {
            db_name: db_name,
            view_name: view_name,
        }
    }

    #[doc(hidden)]
    pub fn parse(s: &'static str) -> Result<Self, Error> {
        Ok(try!(s.into_database_view_path()).into())
    }

    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn view_name(&self) -> &ViewName {
        &self.view_name
    }

    pub fn as_ref(&self) -> DatabaseViewPathRef {
        DatabaseViewPathRef {
            db_name: self.db_name.as_ref(),
            view_name: self.view_name.as_ref(),
        }
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DatabaseViewPathIter {
        self.as_ref().into_iter()
    }
}

impl<'a> From<&'a DatabaseViewPath> for DatabaseViewPathRef<'a> {
    fn from(view_path: &'a DatabaseViewPath) -> Self {
        view_path.as_ref()
    }
}

impl<'a, T, V> From<(T, V)> for DatabaseViewPathRef<'a>
    where T: Into<DatabasePathRef<'a>>,
          V: Into<ViewNameRef<'a>>
{
    fn from(parts: (T, V)) -> Self {
        DatabaseViewPathRef {
            db_name: parts.0.into().db_name,
            view_name: parts.1.into(),
        }
    }
}

impl<'a> From<DatabaseViewPathRef<'a>> for DatabaseViewPath {
    fn from(view_path: DatabaseViewPathRef<'a>) -> Self {
        DatabaseViewPath {
            db_name: view_path.db_name.into(),
            view_name: view_path.view_name.into(),
        }
    }
}

impl<T, V> From<(T, V)> for DatabaseViewPath
    where T: Into<DatabasePath>,
          V: Into<ViewName>
{
    fn from(parts: (T, V)) -> Self {
        DatabaseViewPath {
            db_name: parts.0.into().db_name,
            view_name: parts.1.into(),
        }
    }
}

#[doc(hidden)]
impl<'a> IntoIterator for DatabaseViewPathRef<'a> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = DatabaseViewPathIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DatabaseViewPathIter::DatabaseName(self)
    }
}

pub enum DatabaseViewPathIter<'a> {
    DatabaseName(DatabaseViewPathRef<'a>),
    ViewName(DatabaseViewPathRef<'a>),
    Done,
}

impl<'a> Iterator for DatabaseViewPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self {
            &mut DatabaseViewPathIter::DatabaseName(path) => {
                (path.db_name.inner, DatabaseViewPathIter::ViewName(path))
            }
            &mut DatabaseViewPathIter::ViewName(path) => {
                (path.view_name.inner, DatabaseViewPathIter::Done)
            }
            &mut DatabaseViewPathIter::Done => {
                return None;
            }
        };

        *self = next;
        Some(item)
    }
}

impl<'a> IntoDatabaseViewPath<'a> for &'static str {
    fn into_database_view_path(self) -> Result<DatabaseViewPathRef<'a>, Error> {

        let mut path_extractor = PathExtractor::new(self);
        let db_name = try!(path_extractor.extract_nonfinal());

        let view_name = try!(path_extractor.extract_final());

        Ok(DatabaseViewPathRef {
            db_name: db_name.into(),
            view_name: view_name.into(),
        })
    }
}

impl<'a> IntoDatabaseViewPath<'a> for DatabaseViewPathRef<'a> {
    fn into_database_view_path(self) -> Result<DatabaseViewPathRef<'a>, Error> {
        Ok(self)
    }
}

impl<'a> IntoDatabaseViewPath<'a> for &'a DatabaseViewPath {
    fn into_database_view_path(self) -> Result<DatabaseViewPathRef<'a>, Error> {
        Ok(self.into())
    }
}

impl<'a, T, V> IntoDatabaseViewPath<'a> for (T, V)
    where T: IntoDatabasePath<'a>,
          V: Into<ViewNameRef<'a>>
{
    fn into_database_view_path(self) -> Result<DatabaseViewPathRef<'a>, Error> {
        Ok(DatabaseViewPathRef {
            db_name: try!(self.0.into_database_path()).db_name,
            view_name: self.1.into(),
        })
    }
}

#[cfg(test)]
mod tests {

    use DatabaseViewPathRef;
    use IntoDatabaseViewPath;

    #[test]
    fn into_iter() {
        let view_path = "/foo/_all_docs".into_database_view_path().unwrap();
        let expected = vec!["foo", "_all_docs"];
        let got = view_path.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_ok() {
        let expected = DatabaseViewPathRef {
            db_name: "foo".into(),
            view_name: "_all_docs".into(),
        };
        let got = "/foo/_all_docs".into_database_view_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_tuple_of_static_str_refs() {
        let expected = DatabaseViewPathRef {
            db_name: "foo".into(),
            view_name: "_all_docs".into(),
        };
        let got = ("/foo", "_all_docs").into_database_view_path().unwrap();
        assert_eq!(expected, got);
    }

}
