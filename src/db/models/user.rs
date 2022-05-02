use crate::db::lower;
use crate::db::schema::users;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error;
use diesel::PgConnection;

#[derive(Queryable)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn find(c: &mut PgConnection, username: &str) -> Result<User, Error> {
        use crate::db::schema::users::dsl::{username as username_column, users};

        users
            .filter(lower(username_column).eq(lower(username)))
            .first(c)
    }
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a str,
}
