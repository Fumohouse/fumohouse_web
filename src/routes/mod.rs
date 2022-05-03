use crate::db::models::User;
use rocket::{form::Context, serde::Serialize};

pub mod auth;
pub mod pages;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct EmptyContext {}

#[derive(Default, Serialize)]
pub struct DefaultContext<'a, 'b> {
    user: Option<User>,
    csrf_token: Option<&'a str>,
    captcha_site_key: Option<&'a str>,
    form_context: Option<&'a Context<'b>>,
}
