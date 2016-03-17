use Error;
use super::*;
use super::path_extract_final;

impl<'a> DatabasePath<'a> {
    #[doc(hidden)]
    pub fn database_name(&self) -> &'a DatabaseName {
        self.db_name
    }
}

#[doc(hidden)]
impl<'a> IntoIterator for DatabasePath<'a> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = DatabasePathIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DatabasePathIter::DatabaseName(self)
    }
}

impl DatabasePathBuf {
    #[doc(hidden)]
    pub fn parse(s: &'static str) -> Result<Self, Error> {
        let path = try!(s.into_database_path());
        Ok(DatabasePathBuf { db_name_buf: path.db_name.to_owned() })
    }

    #[doc(hidden)]
    pub fn as_database_path(&self) -> DatabasePath {
        DatabasePath { db_name: &self.db_name_buf }
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DatabasePathIter {
        self.as_database_path().into_iter()
    }
}

pub enum DatabasePathIter<'a> {
    DatabaseName(DatabasePath<'a>),
    Done,
}

impl<'a> Iterator for DatabasePathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self {
            &mut DatabasePathIter::DatabaseName(ref path) => {
                (&path.db_name.inner, DatabasePathIter::Done)
            }
            &mut DatabasePathIter::Done => {
                return None;
            }
        };

        *self = next;
        Some(item)
    }
}

impl<'a> IntoDatabasePath<'a> for &'static str {
    fn into_database_path(self) -> Result<DatabasePath<'a>, Error> {
        let db_name = try!(path_extract_final(self));
        Ok(DatabasePath { db_name: DatabaseName::new(db_name) })
    }
}

impl<'a> IntoDatabasePath<'a> for DatabasePath<'a> {
    fn into_database_path(self) -> Result<DatabasePath<'a>, Error> {
        Ok(self)
    }
}

impl<'a> IntoDatabasePath<'a> for &'a DatabasePathBuf {
    fn into_database_path(self) -> Result<DatabasePath<'a>, Error> {
        Ok(self.as_database_path())
    }
}

impl<'a, T: AsRef<DatabaseName> + ?Sized> IntoDatabasePath<'a> for &'a T {
    fn into_database_path(self) -> Result<DatabasePath<'a>, Error> {
        Ok(DatabasePath { db_name: self.as_ref() })
    }
}

#[cfg(test)]
mod tests {

    use Error;
    use error::PathParseErrorKind;
    use super::super::*;

    #[test]
    fn database_path_into_iter() {
        let got = "/foo".into_database_path().unwrap().into_iter().collect::<Vec<_>>();
        assert_eq!(vec!["foo"], got);
    }

    #[test]
    fn database_path_buf_parse_ok() {
        let expected = DatabasePathBuf { db_name_buf: DatabaseNameBuf::from("foo") };
        let got = DatabasePathBuf::parse("/foo").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn database_path_buf_parse_nok() {
        match DatabasePathBuf::parse("foo") {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_buf_as_database_path() {
        let expected = DatabasePath { db_name: &DatabaseName::new("foo") };
        let db_path_buf = DatabasePathBuf::parse("/foo").unwrap();
        let got = db_path_buf.as_database_path();
        assert_eq!(expected, got);
    }

    #[test]
    fn database_path_buf_iter() {
        let db_path_buf = DatabasePathBuf::parse("/foo").unwrap();
        let got = db_path_buf.iter().collect::<Vec<_>>();
        assert_eq!(vec!["foo"], got);
    }

    #[test]
    fn static_str_ref_into_database_path_ok() {
        let db_name_buf = DatabaseNameBuf::from("foo");
        let expected = DatabasePath { db_name: &db_name_buf };
        let got = "/foo".into_database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_ref_into_database_path_nok_no_leading_slash() {
        match "foo".into_database_path() {
            Err(Error::PathParse(PathParseErrorKind::NoLeadingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_database_path_nok_empty_database_name() {
        match "/".into_database_path() {
            Err(Error::PathParse(PathParseErrorKind::EmptySegment)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_database_path_nok_trailing_slash() {
        match "/foo/".into_database_path() {
            Err(Error::PathParse(PathParseErrorKind::TrailingSlash)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn static_str_ref_into_database_path_nok_too_many_segments() {
        match "/foo/bar".into_database_path() {
            Err(Error::PathParse(PathParseErrorKind::TooManySegments)) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn database_path_buf_into_database_path_ok() {
        let db_name_buf = DatabaseNameBuf::from("foo");
        let db_path_buf = DatabasePathBuf { db_name_buf: db_name_buf.clone() };
        let expected = DatabasePath { db_name: &db_name_buf };
        let got = db_path_buf.into_database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn database_path_into_database_path_ok() {
        let db_name_buf = DatabaseNameBuf::from("foo");
        let db_path = DatabasePath { db_name: &db_name_buf };
        let expected = db_path.clone();
        let got = db_path.into_database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn database_name_buf_into_database_path_ok() {
        let db_name_buf = DatabaseNameBuf::from("foo");
        let expected = DatabasePath { db_name: &db_name_buf };
        let got = db_name_buf.into_database_path().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn static_str_impls_into_database_path() {
        fn f<'a, P: IntoDatabasePath<'a>>(_: P) {}
        f("/foo");
    }

    #[test]
    fn database_path_impls_into_database_path() {
        fn f<'a, P: IntoDatabasePath<'a>>(_: P) {}
        f(&DatabasePathBuf { db_name_buf: DatabaseNameBuf::from("foo") });
    }

    #[test]
    fn database_name_buf_impls_into_database_path() {
        fn f<'a, P: IntoDatabasePath<'a>>(_: P) {}
        f(&DatabaseNameBuf::from("foo"));
    }

    #[test]
    fn database_name_impls_into_database_path() {
        fn f<'a, P: IntoDatabasePath<'a>>(_: P) {}
        f(DatabaseName::new("foo"));
    }
}
