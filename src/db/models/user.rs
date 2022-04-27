use crate::db::schema::users;
use chrono::NaiveDateTime;

#[derive(Queryable)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub creation_date: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a str,
}
