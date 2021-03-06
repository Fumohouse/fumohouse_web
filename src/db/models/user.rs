use crate::db::{lower, schema::users};
use argon2::{Argon2, PasswordHash, password_hash::Error as ArgonError, PasswordVerifier};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, result::Error, PgConnection};
use rocket::serde::Serialize;

#[derive(Queryable, Serialize)]
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

    pub fn verify_password(&self, argon: &Argon2, password: &str) -> Result<(), ArgonError> {
        let db_hash = PasswordHash::new(&self.password)?;

        argon.verify_password(password.as_bytes(), &db_hash)
    }
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a str,
}
