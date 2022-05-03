use crate::util::UserSession;
use rocket::Route;
use rocket_dyn_templates::Template;

pub fn routes() -> Vec<Route> {
    routes![index]
}

#[get("/")]
fn index(user_session: UserSession) -> Template {
    Template::render("index", super::DefaultContext {
        user: user_session.user,
        ..Default::default()
    })
}
