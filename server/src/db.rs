use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::sync::OnceCell;

static DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

/// Initializes the PostgreSQL database connection pool.
///
/// This asynchronous function creates a connection pool using `sqlx`'s `PgPoolOptions`. It retrieves
/// the database URL from the `DATABASE_URL` environment variable and configures the pool with both minimum
/// and maximum connections set to 1. The pool is then stored in a global static variable, ensuring it is
/// initialized only once.
///
/// # Panics
///
/// Panics if the `DATABASE_URL` environment variable is not set or if the pool has already been initialized.
///
/// # Errors
///
/// Returns an `sqlx::Error` if establishing a connection to the database fails.
///
/// # Examples
///
/// ```
/// use sqlx::Error;
///
/// // Set the required environment variable for the database URL.
/// std::env::set_var("DATABASE_URL", "postgres://user:password@localhost/db");
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     // Initialize the pool; this should be done only once.
///     init().await?;
///     Ok(())
/// }
/// ```pub async fn init() -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL should be set"))
        .await?;
    DB_POOL.set(pool).expect("db pool already initialised");
    Ok(())
}

/// Retrieves the globally initialized PostgreSQL connection pool.
///
/// Returns a reference to the database connection pool previously set up by the initializer.
/// 
/// # Panics
///
/// Panics with "db pool not initialised" if the pool has not been initialized prior to this call.
/// 
/// # Examples
///
/// ```
/// // Assume the pool has been initialized using the initializer function.
/// let pool = db::get();
/// // Use `pool` to execute queries with sqlx.
/// ```pub fn get() -> &'static PgPool {
    DB_POOL.get().expect("db pool not initialised")
}
