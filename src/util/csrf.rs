use rocket::{
    http::{Cookie, CookieJar, Status},
    outcome::Outcome::{Failure, Success},
    request::{FromRequest, Outcome, Request},
};
use std::{option::Option, str};

pub const TOKEN_NAME: &str = "csrf_token";
const TOKEN_LENGTH: usize = 64;

quick_error! {
    #[derive(Debug)]
    pub enum CsrfError {
        VerificationFailed {
            display("CSRF verification failed.")
        }
        GenerationFailed {
            display("Failed to generate CSRF token.")
        }
    }
}

pub struct CsrfToken {
    pub token: String,
}

#[rocket::async_trait]
impl<'a> FromRequest<'a> for CsrfToken {
    type Error = CsrfError;

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, Self::Error> {
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

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, Self::Error> {
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
                            CsrfError::GenerationFailed,
                        ))
                    }
                }
            }
        }

        match request.client_ip() {
            Some(ip) => info!("csrf violation from: {}", ip),
            None => info!("csrf violation from unknown ip"),
        }

        return Failure((
            Status::Forbidden,
            CsrfError::VerificationFailed,
        ));
    }
}
