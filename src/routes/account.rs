use std::collections::HashMap;

use super::{BaseData, DefaultContext};
use crate::{
    db::FumohouseDb,
    util::{self, CsrfToken, CsrfVerify, SiteMessages, UserSession},
};
use argon2::Argon2;
use diesel::{prelude::*, result::Error as DieselError};
use rocket::{
    form::{name::NameView, Context, Contextual, Form, FromForm, Options, ValueField},
    response::Redirect,
    Route, State,
};
use rocket_dyn_templates::Template;

pub fn routes() -> Vec<Route> {
    routes![edit_get, edit_post]
}

#[get("/edit")]
fn edit_get(csrf: CsrfToken, user_session: UserSession) -> Result<Template, Redirect> {
    if !user_session.user.is_some() {
        return Err(Redirect::to(uri!("/auth/login")));
    }

    Ok(Template::render(
        "account/edit",
        DefaultContext {
            base: BaseData {
                user: user_session.user,
                csrf_token: &csrf.token,
            },
            captcha_site_key: None,
            form_context: Some(&Context::default()),
        },
    ))
}

#[derive(FromForm)]
struct PasswordChange<'a> {
    current_password: &'a str,
    #[field(validate = len(super::auth::PASSWORD_MIN_LENGTH..))]
    new_password: &'a str,
    verify_password: &'a str,
}

fn parse<'a, T>(body: &'a HashMap<String, String>) -> Contextual<'a, T>
where
    T: FromForm<'a>,
{
    let mut ctx = Contextual::<T>::init(Options { strict: false });

    for (key, value) in body {
        Contextual::<T>::push_value(
            &mut ctx,
            ValueField {
                name: NameView::new(key),
                value,
            },
        )
    }

    let result = Contextual::<T>::finalize(ctx);
    result.unwrap()
}

enum EditResult<'a> {
    Success(Context<'a>),
    SessionInvalidated,
}

async fn handle_edit<'a>(
    conn: &FumohouseDb,
    argon: &Argon2<'_>,
    user_session: &UserSession,
    body: &'a HashMap<String, String>,
) -> Option<EditResult<'a>> {
    use crate::db::schema::{sessions, users};
    use EditResult::*;

    let target: &str = body.get("target")?;

    match target {
        "password" => {
            let user = user_session.user.as_ref().unwrap();
            let mut result = parse::<PasswordChange>(body);

            if let Some(ref form_data) = result.value {
                if user
                    .verify_password(argon, form_data.current_password)
                    .is_err()
                {
                    result
                        .context
                        .push_error(SiteMessages::PasswordIncorrect.into());
                    return Some(Success(result.context));
                }

                if form_data.new_password != form_data.verify_password {
                    result
                        .context
                        .push_error(SiteMessages::PasswordsDontMatch.into());
                    return Some(Success(result.context));
                }

                let hash_result = util::hash_password(&argon, &form_data.new_password);

                match hash_result {
                    Ok(hash) => {
                        let user_id = user.id;

                        let update_result = conn
                            .run(move |c| -> Result<(), DieselError> {
                                diesel::update(users::table.filter(users::id.eq(user_id)))
                                    .set(users::password.eq(hash))
                                    .execute(c)?;

                                diesel::delete(
                                    sessions::table.filter(sessions::user_id.eq(user_id)),
                                )
                                .execute(c)?;

                                Ok(())
                            })
                            .await;

                        if let Err(err) = update_result {
                            result.context.push_error(SiteMessages::GenericError.into());
                            error!("account edit: password update failed: {}", err);
                        }

                        info!(
                            "account edit: {}'s password changed; all sessions invalidated",
                            user.username
                        );

                        return Some(SessionInvalidated);
                    }
                    Err(err) => {
                        result.context.push_error(SiteMessages::GenericError.into());
                        error!("account edit: password hash failed: {}", err);
                    }
                }
            }

            Some(EditResult::Success(result.context))
        }
        _ => None,
    }
}

#[post("/edit", data = "<form>")]
async fn edit_post<'a>(
    csrf: CsrfVerify,
    user_session: UserSession,
    form: Form<HashMap<String, String>>,
    conn: FumohouseDb,
    argon: &State<Argon2<'_>>,
) -> Result<Template, Redirect> {
    if !user_session.user.is_some() {
        return Err(Redirect::to(uri!("/auth/login")));
    }

    let result = handle_edit(&conn, argon, &user_session, &form).await;
    let mut context = None;

    if let Some(edit_result) = result {
        match edit_result {
            EditResult::Success(ctx) => context = Some(ctx),
            EditResult::SessionInvalidated => return Err(Redirect::to(uri!("/auth/login"))),
        }
    }

    Ok(Template::render(
        "account/edit",
        DefaultContext {
            base: BaseData {
                user: user_session.user,
                csrf_token: csrf.new_token(),
            },
            captcha_site_key: None,
            form_context: context.as_ref(),
        },
    ))
}
