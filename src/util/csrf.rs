use rand::distributions::Alphanumeric;
use rand::Rng;
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::Outcome::Success;
use rocket::request::{self, FromRequest, Request};
use std::str;

pub const TOKEN_NAME: &str = "csrf_token";
const TOKEN_LENGTH: usize = 64;

pub struct CsrfToken {
    pub token: String,
    pub token_cookie: Option<String>,
}

#[rocket::async_trait]
impl<'a> FromRequest<'a> for CsrfToken {
    type Error = rocket::Error;

    async fn from_request(request: &'a Request<'_>) -> request::Outcome<Self, Self::Error> {
        let cookies = request.guard::<&CookieJar<'_>>().await.unwrap();

        let existing = match cookies.get_private(TOKEN_NAME) {
            Some(c) => Some(c.value().to_string()),
            None => None,
        };

        let token = gen_token();
        cookies.add_private(Cookie::new(TOKEN_NAME, token.clone()));

        Success(CsrfToken {
            token,
            token_cookie: existing,
        })
    }
}

fn gen_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(TOKEN_LENGTH)
        .map(char::from)
        .collect::<String>()
}
