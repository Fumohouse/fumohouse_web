use crate::db::models::User;
use rocket::{form::Context, serde::Serialize};

pub mod auth;
pub mod pages;

#[derive(Serialize)]
pub struct BaseData<'a> {
    user: Option<User>,
    csrf_token: &'a str,
}

#[derive(Serialize)]
pub struct DefaultContext<'a, 'b> {
    base: BaseData<'a>,
    captcha_site_key: Option<&'a str>,
    form_context: Option<&'a Context<'b>>,
}

impl DefaultContext<'_, '_> {
    pub fn base_only<'a, 'b>(base: BaseData<'a>) -> DefaultContext<'a, 'b> {
        DefaultContext {
            base,
            captcha_site_key: None,
            form_context: None,
        }
    }
}
