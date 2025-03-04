use std::{io, sync::OnceLock};

use tracing::Level;
use tracing_appender::{non_blocking::WorkerGuard, rolling::Rotation};
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::CONFIG;

static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static STDOUT_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static STDERR_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Initializes the logging framework for the application.
///
/// Configures a rolling file logger that rotates daily and writes logs with the suffix
/// "server.aesterisk.log" to the directory specified in the global configuration. This function
/// sets up non-blocking loggers for file output, standard output (logging at DEBUG level), and
/// standard error (logging at WARN level). When compiled with the `tokio_debug` feature, a
/// console subscriber is additionally initialized for enhanced debugging support.
///
/// # Panics
///
/// Panics if any of the logging guards have already been set, ensuring that the logging subsystem
/// is initialized only once.
///
/// # Examples
///
/// ```rust
/// // Initialize the logging framework.
/// my_crate::logging::init();
///
/// // Use tracing macros to log messages.
/// tracing::info!("Application logging initialized successfully.");
/// ```
pub fn init() {
    #[cfg(feature = "tokio_debug")]
    let console_layer = console_subscriber::Builder::default().spawn();

    let logs_rotation = tracing_appender::rolling::Builder::new().filename_suffix("server.aesterisk.log").rotation(Rotation::DAILY).build(&CONFIG.logging.folder).expect("could not initialize file logger");
    let (logs_file, logs_file_guard) = tracing_appender::non_blocking(logs_rotation);
    FILE_GUARD.set(logs_file_guard).expect("logs_file_guard already set");
    let logs_file_layer = tracing_subscriber::fmt::layer().with_writer(logs_file.with_max_level(Level::DEBUG)).with_ansi(false);

    let (logs_stdout, logs_stdout_guard) = tracing_appender::non_blocking(io::stdout());
    STDOUT_GUARD.set(logs_stdout_guard).expect("logs_stdout_guard already set");
    let (logs_stderr, logs_stderr_guard) = tracing_appender::non_blocking(io::stderr());
    STDERR_GUARD.set(logs_stderr_guard).expect("logs_stderr_guard already set");
    let logs_stdout_layer = tracing_subscriber::fmt::layer().with_writer(logs_stderr.with_max_level(Level::WARN).or_else(logs_stdout.with_max_level(Level::DEBUG))).with_ansi(true);

    #[cfg(feature = "tokio_debug")]
    tracing_subscriber::registry()
        .with(console_layer)
        .with(logs_file_layer)
        .with(logs_stdout_layer)
        .init();

    #[cfg(not(feature = "tokio_debug"))]
    tracing_subscriber::registry()
        .with(logs_file_layer)
        .with(logs_stdout_layer)
        .init();
}
