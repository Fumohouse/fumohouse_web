use super::{BaseData, DefaultContext};
use crate::db::{
    models::{NewUser, User},
    FumohouseDb,
};
use crate::util::{
    CaptchaVerifier, CsrfToken, CsrfVerify, SessionUtils, SiteMessages, UserSession,
};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use diesel::{prelude::*, result::Error as DieselError};
use rocket::form::{Context, Contextual, Error, Form};
use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
use rocket::{Route, State};
use rocket_dyn_templates::Template;

pub fn routes() -> Vec<Route> {
    routes![register_get, register_post, login_get, login_post, logout]
}

fn valid_char(c: char) -> bool {
    char::is_ascii_alphanumeric(&c)
        || match c {
            // a few symbols that are ok
            ' ' => true,
            '[' | ']' | '(' | ')' => true,
            '-' | '_' => true,
            _ => false,
        }
}

#[derive(FromForm)]
struct RegisterForm<'a> {
    #[field(validate = len(1..))]
    #[field(validate = with(|u| u.chars().all(valid_char), SiteMessages::UsernameInvalid.description()))]
    username: &'a str,
    #[field(validate = len(8..))]
    password: &'a str,
    #[field(name = "h-captcha-response")]
    captcha_response: &'a str,
}

#[get("/register")]
fn register_get(
    user_session: UserSession,
    csrf: CsrfToken,
    captcha: &State<CaptchaVerifier>,
) -> Result<Template, Redirect> {
    if user_session.user.is_some() {
        return Err(Redirect::to(uri!("/")));
    }

    Ok(Template::render(
        "register",
        DefaultContext {
            base: BaseData {
                user: None,
                csrf_token: &csrf.token,
            },
            captcha_site_key: Some(&captcha.site_key),
            form_context: Some(&Context::default()),
        },
    ))
}

async fn handle_register<'a>(
    conn: &FumohouseDb,
    argon: &Argon2<'_>,
    form_data: &RegisterForm<'a>,
    errors: &mut Vec<Error<'_>>,
) -> Option<User> {
    use crate::db::schema::users;

    let requested_username = form_data.username.to_string();

    let existing = conn.run(move |c| User::find(c, &requested_username)).await;

    if !existing.is_err() {
        errors.push(SiteMessages::UsernameInUse.into());
        return None;
    } else if let Err(err) = existing {
        if err != DieselError::NotFound {
            errors.push(Error::validation(SiteMessages::GenericError.to_string()));
            error!(
                "registration: diesel errored when trying to find user: {}",
                err
            );
            return None;
        }
    }

    let requested_username = form_data.username.to_string();
    let salt = SaltString::generate(&mut OsRng);
    let hashed_pass = argon.hash_password(form_data.password.as_bytes(), &salt);

    match hashed_pass {
        Ok(hash) => {
            let hash = hash.to_string();

            let new_user = conn
                .run(move |c| {
                    let new_user = NewUser {
                        username: &requested_username,
                        password: &hash.to_string(),
                    };

                    diesel::insert_into(users::table)
                        .values(&new_user)
                        .get_result::<User>(c)
                })
                .await
                .ok()?;

            info!("registration: new user: {}", new_user.username);

            return Some(new_user);
        }
        Err(err) => {
            errors.push(SiteMessages::GenericError.into());
            error!("registration: hash failed: {}", err);
        }
    }

    None
}

#[post("/register", data = "<form>")]
async fn register_post<'a>(
    csrf: CsrfVerify,
    mut form: Form<Contextual<'a, RegisterForm<'a>>>,
    captcha: &State<CaptchaVerifier>,
    argon: &State<Argon2<'_>>,
    conn: FumohouseDb,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, (Status, Template)> {
    // Errors are added all at once at the end of the request
    // to avoid issues with mutable references
    let mut errors = Vec::new();

    if let Some(ref form_data) = form.value {
        let captcha_success = captcha
            .verify(form_data.captcha_response)
            .await
            .unwrap_or_else(|err| {
                errors.push(SiteMessages::GenericError.into());
                error!("registration: captcha verification failed: {}", err);

                false
            });

        if captcha_success {
            let result = handle_register(&conn, &argon, form_data, &mut errors).await;

            if let Some(user) = result {
                SessionUtils::begin_session(&user, &conn, cookies)
                    .await
                    .unwrap_or_else(|err| {
                        error!("registration: failed to start user session: {}", err);
                    });

                return Ok(Redirect::to(uri!("/")));
            }
        } else {
            errors.push(SiteMessages::CAPTCHAFailed.into());
        }
    }

    form.context.push_errors(errors);

    Err((
        form.context.status(),
        Template::render(
            "register",
            DefaultContext {
                base: BaseData {
                    user: None,
                    csrf_token: csrf.new_token(),
                },
                captcha_site_key: Some(&captcha.site_key),
                form_context: Some(&form.context),
            },
        ),
    ))
}

#[derive(FromForm)]
struct LoginForm<'a> {
    username: &'a str,
    password: &'a str,
}

#[get("/login")]
async fn login_get(user_session: UserSession, csrf: CsrfToken) -> Result<Template, Redirect> {
    if user_session.user.is_some() {
        return Err(Redirect::to(uri!("/")));
    }

    Ok(Template::render(
        "login",
        DefaultContext {
            base: BaseData {
                user: None,
                csrf_token: &csrf.token,
            },
            captcha_site_key: None,
            form_context: Some(&Context::default()),
        },
    ))
}

async fn handle_login<'a>(
    conn: &FumohouseDb,
    argon: &Argon2<'_>,
    form_data: &LoginForm<'a>,
) -> Option<User> {
    use argon2::{password_hash::PasswordHash, PasswordVerifier};

    let username = form_data.username.to_string();
    let user = conn.run(move |c| User::find(c, &username)).await.ok()?;

    let db_hash = PasswordHash::new(&user.password).ok()?;

    if argon
        .verify_password(form_data.password.as_bytes(), &db_hash)
        .is_ok()
    {
        info!("login: new login: {}", user.username);
        return Some(user);
    }

    None
}

#[post("/login", data = "<form>")]
async fn login_post<'a>(
    csrf: CsrfVerify,
    mut form: Form<Contextual<'a, LoginForm<'a>>>,
    argon: &State<Argon2<'_>>,
    conn: FumohouseDb,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, (Status, Template)> {
    let mut errors: Vec<Error> = Vec::new();

    if let Some(ref form_data) = form.value {
        let result = handle_login(&conn, &argon, form_data).await;

        match result {
            Some(u) => {
                SessionUtils::begin_session(&u, &conn, cookies)
                    .await
                    .unwrap_or_else(|err| {
                        error!("login: failed to start user session: {}", err);
                    });

                return Ok(Redirect::to(uri!("/")));
            }
            None => errors.push(SiteMessages::LoginFailed.into()),
        }
    }

    form.context.push_errors(errors);

    Err((
        form.context.status(),
        Template::render(
            "login",
            DefaultContext {
                base: BaseData {
                    user: None,
                    csrf_token: csrf.new_token(),
                },
                form_context: Some(&form.context),
                captcha_site_key: None,
            },
        ),
    ))
}

#[post("/logout")]
async fn logout(
    _csrf: CsrfVerify,
    user_session: UserSession,
    conn: FumohouseDb,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    if let Some(session) = user_session.session {
        if let Err(err) = SessionUtils::end_session(&conn, cookies, &session).await {
            error!("logout: failed to end user session: {}", err);
            return Err(Status::InternalServerError);
        }

        info!("logout: {} logged out", user_session.user.unwrap().username);
    }


    Ok(Redirect::to(uri!("/auth/login")))
}