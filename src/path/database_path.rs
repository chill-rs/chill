use Error;
use super::PathExtractor;
use super::*;

impl<'a> DatabasePathRef<'a> {
    pub fn database_name(&self) -> &DatabaseNameRef<'a> {
        &self.db_name
    }
}

impl DatabasePath {
    pub fn database_name(&self) -> &DatabaseName {
        &self.db_name
    }

    pub fn as_ref(&self) -> DatabasePathRef {
        DatabasePathRef { db_name: self.db_name.as_ref() }
    }

    #[doc(hidden)]
    pub fn iter(&self) -> DatabasePathIter {
        self.as_ref().into_iter()
    }
}

impl<'a> From<&'a DatabasePath> for DatabasePathRef<'a> {
    fn from(db_path: &'a DatabasePath) -> Self {
        DatabasePathRef { db_name: db_path.db_name.as_ref() }
    }
}

impl<'a> From<DatabasePathRef<'a>> for DatabasePath {
    fn from(db_path: DatabasePathRef<'a>) -> Self {
        DatabasePath { db_name: db_path.db_name.into() }
    }
}

#[doc(hidden)]
impl<'a> IntoIterator for DatabasePathRef<'a> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = DatabasePathIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DatabasePathIter::DatabaseName(self)
    }
}

pub enum DatabasePathIter<'a> {
    DatabaseName(DatabasePathRef<'a>),
    Done,
}

impl<'a> Iterator for DatabasePathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self {
            &mut DatabasePathIter::DatabaseName(ref path) => {
                (path.db_name.inner as &str, DatabasePathIter::Done)
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
    fn into_database_path(self) -> Result<DatabasePathRef<'a>, Error> {
        let db_name = try!(PathExtractor::new(self).extract_final());
        Ok(DatabasePathRef { db_name: db_name.into() })
    }
}

impl<'a> IntoDatabasePath<'a> for DatabasePathRef<'a> {
    fn into_database_path(self) -> Result<DatabasePathRef<'a>, Error> {
        Ok(self)
    }
}

impl<'a> IntoDatabasePath<'a> for &'a DatabasePath {
    fn into_database_path(self) -> Result<DatabasePathRef<'a>, Error> {
        Ok(self.into())
    }
}

impl<'a> IntoDatabasePath<'a> for DatabaseNameRef<'a> {
    fn into_database_path(self) -> Result<DatabasePathRef<'a>, Error> {
        Ok(DatabasePathRef { db_name: self })
    }
}

impl<'a> IntoDatabasePath<'a> for &'a DatabaseName {
    fn into_database_path(self) -> Result<DatabasePathRef<'a>, Error> {
        Ok(DatabasePathRef { db_name: self.as_ref() })
    }
}

#[cfg(test)]
mod tests {

    use super::super::*;

    #[test]
    fn into_iter() {
        let db_path = DatabasePathRef { db_name: "foo".into() };
        let expected = vec!["foo"];
        let got = db_path.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_static_str_ref_ok() {
        let expected = DatabasePathRef { db_name: "foo".into() };
        let got = "/foo".into_database_path().unwrap();
        assert_eq!(expected, got);
    }
}
