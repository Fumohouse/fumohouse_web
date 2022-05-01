use rocket_sync_db_pools::database;
use diesel::sql_types::Text;
use diesel::sql_function;

pub mod models;
pub mod schema;

#[database("fumohouse_db")]
pub struct FumohouseDb(diesel::PgConnection);

sql_function!(fn lower(x: Text) -> Text);
