#[macro_use] extern crate rocket;

use std::env;
use rocket::figment::{value::{Map, Value}, util::map};

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    if cfg!(debug_assertions) {
        println!("Loading environment variables from .env.");
        if let Err(e) = dotenvy::dotenv() {
            println!("Failed to load .env: {}", e);
        }
    }

    let database_url = env::var("DATABASE_URL").expect("The DATABASE_URL environment variable is not set.");

    let db: Map<_, Value> = map! {
        "url" => database_url.into(),
        "pool_size" => 10.into(),
    };

    let figment = rocket::Config::figment()
        .merge(("databases", map!["fumohouse_db" => db]));

    rocket::custom(figment)
        .mount("/", routes![index])
}
