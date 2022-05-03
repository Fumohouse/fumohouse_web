use crate::db::schema::sessions;
use chrono::{DateTime, Utc, Duration};

#[derive(Queryable)]
pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub session_id: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub modified_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    pub fn since_last_modify(&self) -> Duration {
        Utc::now() - self.last_modify()
    }

    pub fn last_modify(&self) -> DateTime<Utc> {
        match self.modified_at {
            Some(date_time) => date_time,
            None => self.created_at,
        }
    }
}

#[derive(Insertable)]
#[table_name = "sessions"]
pub struct NewSession<'a> {
    pub user_id: i64,
    pub session_id: &'a [u8],
    pub expires_at: DateTime<Utc>,
}
