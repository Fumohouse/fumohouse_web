use crate::util::{CaptchaVerifier, CsrfToken, CsrfVerify};
use rocket::form::{Context, Contextual, Error, Form};
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::serde::Serialize;
use rocket::{Route, State};
use rocket_dyn_templates::Template;

pub fn routes() -> Vec<Route> {
    routes![register_get, register_post]
}

#[derive(FromForm)]
pub struct RegisterForm<'a> {
    #[field(validate = len(1..))]
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

#[post("/register", data = "<form>")]
async fn register_post<'a>(
    csrf: CsrfVerify,
    mut form: Form<Contextual<'a, RegisterForm<'a>>>,
    captcha: &State<CaptchaVerifier>,
) -> Result<(Status, Template), Redirect> {
    if csrf.success() {
        if let Some(ref form_data) = form.value {
            let captcha_success = captcha
                .verify(form_data.captcha_response)
                .await
                .unwrap_or_else(|err| {
                    form.context.push_error(Error::validation(format!(
                        "CAPTCHA verification failed: {}",
                        err.to_string()
                    )));

                    false
                });

            if captcha_success {
                // TODO: Account creation
                println!("Hi!");

                // TODO: Redirect to account page; log in
                return Err(Redirect::to(uri!("/")));
            } else {
                form.context
                    .push_error(Error::validation("Incorrect CAPTCHA response"));
            }
        }
    }

    Ok((
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
