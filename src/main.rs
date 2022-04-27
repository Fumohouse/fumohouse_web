#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
extern crate rand;
extern crate serde;

use rocket::figment::{
    util::map,
    value::{Map, Value},
};
use rocket::fs::FileServer;
use rocket_dyn_templates::Template;
use rocket_sync_db_pools::database;
use std::env;

mod db;
mod routes;
mod util;

pub use db::models;

#[database("fumohouse_db")]
struct FumohouseDb(diesel::PgConnection);

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
        .attach(FumohouseDb::fairing())
        .attach(Template::fairing())
        .manage(util::CaptchaVerifier::new())
        .mount("/", FileServer::from("static/"))
        .mount("/", routes::pages::routes())
        .mount("/auth", routes::auth::routes())
}
