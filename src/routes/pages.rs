use rocket::Route;
use rocket_dyn_templates::Template;

pub fn routes() -> Vec<Route> {
    routes![index]
}

#[get("/")]
fn index() -> Template {
    Template::render("index", &super::EmptyContext {})
}
