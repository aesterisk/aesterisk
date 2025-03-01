use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::sync::OnceCell;

static DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

pub async fn init() -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL should be set"))
        .await?;
    DB_POOL.set(pool).expect("db pool already initialised");
    Ok(())
}

pub fn get() -> &'static PgPool {
    DB_POOL.get().expect("db pool not initialised")
}
