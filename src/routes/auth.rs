use super::DefaultContext;
use crate::db::{
    models::{NewUser, User},
    FumohouseDb,
};
use crate::util::{CaptchaVerifier, CsrfToken, CsrfVerify, SessionUtils};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use diesel::prelude::*;
use rocket::form::{Context, Contextual, Error, Form};
use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
use rocket::{Route, State};
use rocket_dyn_templates::Template;

pub fn routes() -> Vec<Route> {
    routes![register_get, register_post, login_get, login_post]
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
    #[field(validate = with(|u| u.chars().all(valid_char), "Username contains invalid characters"))]
    username: &'a str,
    #[field(validate = len(8..))]
    password: &'a str,
    #[field(name = "h-captcha-response")]
    captcha_response: &'a str,
}

#[get("/register")]
fn register_get(csrf: CsrfToken, captcha: &State<CaptchaVerifier>) -> Template {
    Template::render(
        "register",
        DefaultContext {
            csrf_token: Some(&csrf.token),
            captcha_site_key: Some(&captcha.site_key),
            form_context: Some(&Context::default()),
        },
    )
}

async fn handle_register<'a>(
    conn: &FumohouseDb,
    argon: &Argon2<'_>,
    form_data: &RegisterForm<'a>,
    errors: &mut Vec<Error<'_>>,
) -> Option<User> {
    use crate::db::schema::users;

    let requested_username = form_data.username.to_string();

    let existing = conn
        .run(move |c| User::find(c, &requested_username))
        .await;

    if !existing.is_err() {
        errors.push(Error::validation("Username is in use").with_name("username"));
        return None;
    } else if let Err(e) = existing {
        if e != diesel::result::Error::NotFound {
            errors.push(Error::validation("An internal error occurred. Try again or contact the site admin."));
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

            return Some(new_user);
        }
        Err(err) => {
            errors.push(Error::validation(
                "Failed to hash password. Please contact site admin.",
            ));
            println!("Hash failed: {}", err);
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
                errors.push(Error::validation(format!(
                    "CAPTCHA verification failed: {}",
                    err.to_string()
                )));

                false
            });

        if captcha_success {
            let result = handle_register(&conn, &argon, form_data, &mut errors).await;

            if let Some(user) = result {
                SessionUtils::begin_session(&user, &conn, cookies)
                    .await
                    .unwrap_or_else(|e| {
                        println!("Failed to start user session: {e}");
                    });

                return Ok(Redirect::to(uri!("/")));
            }
        } else {
            errors.push(Error::validation("Incorrect CAPTCHA response"));
        }
    }

    form.context.push_errors(errors);

    Err((
        form.context.status(),
        Template::render(
            "register",
            DefaultContext {
                csrf_token: Some(csrf.new_token()),
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
async fn login_get(csrf: CsrfToken) -> Template {
    Template::render(
        "login",
        DefaultContext {
            csrf_token: Some(&csrf.token),
            form_context: Some(&Context::default()),
            ..Default::default()
        },
    )
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
    let mut errors = Vec::new();

    if let Some(ref form_data) = form.value {
        let result = handle_login(&conn, &argon, form_data).await;

        match result {
            Some(u) => {
                SessionUtils::begin_session(&u, &conn, cookies)
                    .await
                    .unwrap_or_else(|e| {
                        println!("Failed to start user session: {}", e);
                    });

                return Ok(Redirect::to(uri!("/")));
            }
            None => errors.push(Error::validation("Invalid username or password")),
        }
    }

    form.context.push_errors(errors);

    Err((
        form.context.status(),
        Template::render(
            "login",
            DefaultContext {
                csrf_token: Some(&csrf.new_token()),
                form_context: Some(&form.context),
                ..Default::default()
            },
        ),
    ))
}
