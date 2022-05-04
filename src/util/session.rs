use crate::db::{
    models::{NewSession, Session, User},
    FumohouseDb,
};
use chrono::{offset::Utc, DateTime, Duration as ChronoDuration};
use diesel::{prelude::*, result::Error as DieselError};
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::{Cookie, CookieJar, Status},
    outcome::Outcome::{Failure, Success},
    request::{FromRequest, Outcome, Request},
    time::{Duration as CookieDuration, OffsetDateTime},
    Rocket,
};
use std::error::Error;

const SESSION_COOKIE_NAME: &str = "fh_session";
const SESSION_ID_LENGTH: usize = 32;
const SESSION_RENEW: i64 = 15; // minutes
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
            time::{self, Duration as TokioDuration},
        };

        let conn = FumohouseDb::get_one(rocket).await.unwrap();

        tokio::spawn(async move {
            let mut interval = time::interval(TokioDuration::from_secs(SESSION_PURGE));

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
    fn new_session_id() -> (String, Vec<u8>) {
        let session_id = super::rand_string(SESSION_ID_LENGTH);
        let hash = super::sha256(&session_id);
        (session_id, hash)
    }

    fn chrono_expiry_now() -> DateTime<Utc> {
        Utc::now() + ChronoDuration::hours(SESSION_EXPIRY)
    }

    fn set_cookie(cookies: &CookieJar<'_>, session_id: String) {
        let mut cookie = Cookie::new(SESSION_COOKIE_NAME, session_id);
        cookie.set_expires(OffsetDateTime::now_utc() + CookieDuration::hours(SESSION_EXPIRY));

        cookies.add_private(cookie);
    }

    pub async fn begin_session(
        user: &User,
        conn: &FumohouseDb,
        cookies: &CookieJar<'_>,
    ) -> Result<(), Box<dyn Error>> {
        use crate::db::schema::sessions;

        let user_id = user.id;
        let (session_id, hash) = Self::new_session_id();

        conn.run(move |c| {
            let new_session = NewSession {
                user_id,
                session_id: &hash,
                expires_at: Self::chrono_expiry_now(),
            };

            diesel::insert_into(sessions::table)
                .values(&new_session)
                .get_result::<Session>(c)
        })
        .await?;

        Self::set_cookie(cookies, session_id);

        Ok(())
    }

    async fn renew_session(
        conn: &FumohouseDb,
        cookies: &CookieJar<'_>,
        session_primary_key: i64,
    ) -> Result<(), DieselError> {
        use crate::db::schema::sessions::dsl::*;

        let (new_sid, new_hash) = Self::new_session_id();

        conn.run(move |c| {
            diesel::update(sessions.filter(id.eq(session_primary_key)))
                .set((
                    session_id.eq(new_hash),
                    modified_at.eq(Utc::now()),
                    expires_at.eq(Self::chrono_expiry_now()),
                ))
                .execute(c)
        })
        .await?;

        Self::set_cookie(cookies, new_sid);

        Ok(())
    }

    pub async fn end_session(
        conn: &FumohouseDb,
        cookies: &CookieJar<'_>,
        session: &Session,
    ) -> Result<(), DieselError> {
        use crate::db::schema::sessions::dsl::*;

        let id_to_delete = session.id;

        conn.run(move |c| {
            diesel::delete(sessions)
                .filter(id.eq(id_to_delete))
                .execute(c)
        }).await?;

        cookies.remove_private(Cookie::named(SESSION_COOKIE_NAME));

        Ok(())
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum SessionError {
        RetrieveFailed { diesel_error: DieselError } {
            display("Failed to retrieve user session information: {}.", diesel_error)
        }
        RenewFailed { diesel_error: DieselError } {
            display("Failed to renew user session: {}.", diesel_error)
        }
        SessionGuardFailed {
            display("The UserSession guard failed.")
        }
        Forbidden {
            display("The user is forbidden from accessing this route.")
        }
    }
}

#[derive(Default)]
pub struct UserSession {
    pub user: Option<User>,
    pub session: Option<Session>,
}

#[rocket::async_trait]
impl<'a> FromRequest<'a> for UserSession {
    type Error = SessionError;

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        use crate::db::schema::{
            sessions::{self, dsl::*},
            users,
        };

        let cookies = request.guard::<&CookieJar<'_>>().await.unwrap();
        let conn = request.guard::<FumohouseDb>().await.unwrap();

        let session_cookie = match cookies.get_private(SESSION_COOKIE_NAME) {
            Some(cookie) => cookie,
            None => return Success(UserSession::default()),
        };

        let token_hash = super::sha256(session_cookie.value());

        let result = conn
            .run(move |c| {
                sessions
                    .filter(session_id.eq(token_hash))
                    .inner_join(users::table)
                    .select((users::all_columns, sessions::all_columns))
                    .first::<(User, Session)>(c)
            })
            .await;

        match result {
            Ok((user, session)) => {
                if session.since_last_modify().num_minutes() > SESSION_RENEW {
                    let renew_result =
                        SessionUtils::renew_session(&conn, cookies, session.id).await;

                    if let Err(diesel_error) = renew_result {
                        return Failure((
                            Status::InternalServerError,
                            SessionError::RenewFailed { diesel_error },
                        ));
                    }

                    info!("session: renewed session of {}", user.username);
                }

                return Success(UserSession {
                    user: Some(user),
                    session: Some(session),
                });
            }
            Err(diesel_error) => match diesel_error {
                DieselError::NotFound => Success(UserSession::default()),
                _ => Failure((
                    Status::InternalServerError,
                    SessionError::RetrieveFailed { diesel_error },
                )),
            },
        }
    }
}
