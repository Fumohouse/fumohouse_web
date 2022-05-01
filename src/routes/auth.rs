use crate::db::{
    models::{NewUser, User},
    FumohouseDb,
};
use crate::util::{CaptchaVerifier, CsrfToken, CsrfVerify};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use diesel::prelude::*;
use rocket::form::{Context, Contextual, Error, Form};
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::serde::Serialize;
use rocket::{Route, State};
use rocket_dyn_templates::Template;

pub fn routes() -> Vec<Route> {
    routes![register_get, register_post]
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
pub struct RegisterForm<'a> {
    #[field(validate = len(1..))]
    #[field(validate = with(|u| u.chars().all(valid_char), "Username contains invalid characters"))]
    username: &'a str,
    #[field(validate = len(8..))]
    password: &'a str,
    #[field(name = "h-captcha-response")]
    captcha_response: &'a str,
}

#[derive(Serialize)]
struct RegisterContext<'a, 'b> {
    csrf_token: &'a str,
    site_key: &'a str,
    // IDK man
    form_context: &'a Context<'b>,
}

#[get("/register")]
fn register_get(csrf: CsrfToken, captcha: &State<CaptchaVerifier>) -> Template {
    Template::render(
        "register",
        RegisterContext {
            csrf_token: &csrf.token,
            site_key: &captcha.site_key,
            form_context: &Context::default(),
        },
    )
}

async fn handle_register<'a>(
    conn: &FumohouseDb,
    argon: &Argon2<'_>,
    form_data: &RegisterForm<'a>,
    errors: &mut Vec<Error<'_>>,
) -> Option<User> {
    use crate::db::lower;
    use crate::db::schema::users::{self, dsl::*};

    let requested_username = form_data.username.to_string();
    let username_lower = requested_username.to_lowercase();

    let existing = conn
        .run(|c| {
            users
                .filter(lower(username).eq(username_lower))
                .limit(1)
                .load::<User>(c)
        })
        .await
        .ok()?;

    if existing.len() > 0 {
        errors.push(Error::validation("Username is in use").with_name("username"));
        return None;
    }

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

            if let Some(_user) = result {
                // TODO: login here
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
            RegisterContext {
                csrf_token: csrf.new_token(),
                site_key: &captcha.site_key,
                form_context: &form.context,
            },
        ),
    ))
}
