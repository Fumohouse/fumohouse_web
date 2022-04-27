use rocket::serde::Serialize;

pub mod auth;
pub mod pages;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct EmptyContext {}
