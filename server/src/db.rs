use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::sync::OnceCell;

static DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

/// Initialise the database connection pool.
pub async fn init() -> Result<(), String> {
    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .connect(&std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL should be set")?)
        .await
        .map_err(|e| format!("SQLx error: {}", e))?;
    DB_POOL.set(pool).map_err(|_| "Database pool already initialised")?;
    Ok(())
}

/// Get the database connection pool.
pub fn get() -> Result<&'static PgPool, &'static str> {
    DB_POOL.get().ok_or("Database pool not initialised")
}
