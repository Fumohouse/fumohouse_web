use super::{BaseData, DefaultContext};
use crate::util::{CsrfToken, UserSession};
use rocket::Route;
use rocket_dyn_templates::Template;

pub fn routes() -> Vec<Route> {
    routes![index]
}

#[get("/")]
fn index(csrf: CsrfToken, user_session: UserSession) -> Template {
    Template::render("index", DefaultContext::base_only(BaseData {
        user: user_session.user,
        csrf_token: &csrf.token,
    }))
}
