use rocket::http::{Cookie, CookieJar, Status};
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::{self, FromRequest, Request};
use std::error::Error;
use std::fmt::{self, Display};
use std::{option::Option, str};

pub const TOKEN_NAME: &str = "csrf_token";
const TOKEN_LENGTH: usize = 64;

#[derive(Debug)]
pub struct CsrfError(String);

impl Error for CsrfError {}

impl Display for CsrfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct CsrfToken {
    pub token: String,
}

#[rocket::async_trait]
impl<'a> FromRequest<'a> for CsrfToken {
    type Error = CsrfError;

    async fn from_request(request: &'a Request<'_>) -> request::Outcome<Self, Self::Error> {
        let cookies = request.guard::<&CookieJar<'_>>().await.unwrap();

        let token = super::rand_string(TOKEN_LENGTH);
        // TODO: The cookie should be marked .secure(true) in production
        cookies.add_private(Cookie::new(TOKEN_NAME, token.clone()));

        Success(CsrfToken { token })
    }
}

pub struct CsrfVerify {
    new_token: String,
}

impl CsrfVerify {
    pub fn new_token(&self) -> &str {
        &self.new_token
    }

    fn get_query<'a>(request: &'a Request<'_>) -> Option<&'a str> {
        let query = request
            .uri()
            .query()?
            .segments()
            .skip_while(|seg| seg.0 != TOKEN_NAME)
            .next()?;

        Some(query.1)
    }

    async fn compare_tokens<'a>(request: &'a Request<'_>) -> Option<bool> {
        let cookies = request.guard::<&CookieJar<'_>>().await.unwrap();

        let query_token = Self::get_query(request)?;
        let cookie = cookies.get_private(TOKEN_NAME)?;

        Some(query_token == cookie.value())
    }
}

#[rocket::async_trait]
impl<'a> FromRequest<'a> for CsrfVerify {
    type Error = CsrfError;

    async fn from_request(request: &'a Request<'_>) -> request::Outcome<Self, Self::Error> {
        let result = Self::compare_tokens(request).await;

        if let Some(success) = result {
            if success {
                match request.guard::<CsrfToken>().await {
                    Success(csrf) => {
                        return Success(CsrfVerify {
                            new_token: csrf.token,
                        })
                    }
                    _ => {
                        return Failure((
                            Status::InternalServerError,
                            CsrfError("Failed to generate CSRF token.".to_string()),
                        ))
                    }
                }
            }
        }

        // TODO: Proper logging?
        if let Some(ip) = request.client_ip() {
            println!("CSRF violation from {}", ip);
        } else {
            println!("CSRF violation, unknown IP");
        }

        return Failure((
            Status::Forbidden,
            CsrfError("CSRF verification failed.".to_string()),
        ));
    }
}
