#[macro_use]
extern crate rocket;

use rocket::figment::{
    util::map,
    value::{Map, Value},
};
use rocket::fs::FileServer;
use rocket::serde::Serialize;
use rocket_dyn_templates::Template;
use std::env;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct EmptyContext {}

#[get("/")]
fn index() -> Template {
    Template::render("index", &EmptyContext {})
}

#[launch]
fn rocket() -> _ {
    if cfg!(debug_assertions) {
        println!("Loading environment variables from .env.");
        if let Err(e) = dotenvy::dotenv() {
            println!("Failed to load .env: {}", e);
        }
    }

    let database_url =
        env::var("DATABASE_URL").expect("The DATABASE_URL environment variable is not set.");

    let db: Map<_, Value> = map! {
        "url" => database_url.into(),
        "pool_size" => 10.into(),
    };

    let figment = rocket::Config::figment().merge(("databases", map!["fumohouse_db" => db]));

    rocket::custom(figment)
        .attach(Template::fairing())
        .mount("/", FileServer::from("static/"))
        .mount("/", routes![index])
}
