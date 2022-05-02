#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
extern crate rand;
extern crate serde;

use argon2::Argon2;
use rocket::figment::{
    util::map,
    value::{Map, Value},
};
use rocket::fs::FileServer;
use rocket_dyn_templates::Template;
use std::env;

mod db;
mod routes;
mod util;

pub use db::models;
use util::SessionUtils;

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
        .attach(db::FumohouseDb::fairing())
        .attach(Template::fairing())
        .attach(SessionUtils::fairing())
        .manage(util::CaptchaVerifier::new())
        .manage(Argon2::default())
        .mount("/", FileServer::from("static/"))
        .mount("/", routes::pages::routes())
        .mount("/auth", routes::auth::routes())
}
