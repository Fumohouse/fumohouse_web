// Three different types of `Duration` are used in this file. Beware!
use crate::db::{
    models::{NewSession, Session, User},
    FumohouseDb,
};
use chrono::{offset::Utc, Duration};
use diesel::prelude::*;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Cookie, CookieJar};
use rocket::time::{Duration as TDuration, OffsetDateTime};
use rocket::Rocket;
use std::error::Error;

const SESSION_COOKIE_NAME: &str = "fh_session";
const SESSION_ID_LENGTH: usize = 32;
const SESSION_EXPIRY: i64 = 30 * 24; // hours

const SESSION_PURGE: u64 = 30 * 60; // seconds
pub struct SessionUtils;

#[rocket::async_trait]
impl Fairing for SessionUtils {
    fn info(&self) -> Info {
        Info {
            name: "purge expired sessions",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<rocket::Orbit>) {
        use crate::db::schema::sessions::dsl::*;
        use rocket::tokio::{
            self,
            time::{self, Duration},
        };

        let conn = FumohouseDb::get_one(rocket).await.unwrap();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(SESSION_PURGE));

            loop {
                interval.tick().await;

                let result = conn
                    .run(|c| {
                        let now = Utc::now();
                        diesel::delete(sessions.filter(expires_at.lt(now))).execute(c)
                    })
                    .await;

                match result {
                    Ok(count) => info!("session: purged {} expired sessions", count),
                    Err(err) => error!("fairing: error purging sessions: {}", err),
                }
            }
        });
    }
}

impl SessionUtils {
    pub async fn begin_session(
        user: &User,
        conn: &FumohouseDb,
        cookies: &CookieJar<'_>,
    ) -> Result<(), Box<dyn Error>> {
        use crate::db::schema::sessions;

        let user_id = user.id;
        let session_id = super::rand_string(SESSION_ID_LENGTH);
        let hash = super::sha256(&session_id);

        conn.run(move |c| {
            let new_session = NewSession {
                user_id,
                session_id: &hash,
                expires_at: Utc::now() + Duration::hours(SESSION_EXPIRY),
            };

            diesel::insert_into(sessions::table)
                .values(&new_session)
                .get_result::<Session>(c)
        })
        .await?;

        let mut cookie = Cookie::new(SESSION_COOKIE_NAME, session_id);
        cookie.set_expires(OffsetDateTime::now_utc() + TDuration::hours(SESSION_EXPIRY));

        cookies.add_private(cookie);

        Ok(())
    }
}
