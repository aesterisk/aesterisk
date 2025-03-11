use std::{io, sync::Mutex};

use tracing::{subscriber::DefaultGuard, Level};
use tracing_appender::{non_blocking::WorkerGuard, rolling::Rotation};
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, Layer};

use crate::config;

static FILE_GUARD: Mutex<Option<WorkerGuard>> = Mutex::new(None);
static STDERR_GUARD: Mutex<Option<WorkerGuard>> = Mutex::new(None);
static STDOUT_GUARD: Mutex<Option<WorkerGuard>> = Mutex::new(None);
static SUBSCRIBER_GUARD: Mutex<Option<DefaultGuard>> = Mutex::new(None);

/// Initialize the logging system. The configuration must be loaded before calling this function.
pub fn init() {
    let config = config::get().expect("config is not initialized");

    let logs_rotation = tracing_appender::rolling::Builder::new().filename_suffix("daemon.aesterisk.log").rotation(Rotation::DAILY).build(&config.logging.folder).expect("could not initialize file logger");
    let (logs_file, logs_file_guard) = tracing_appender::non_blocking(logs_rotation);
    FILE_GUARD.lock().expect("file_guard poisoned").replace(logs_file_guard);
    let logs_file_layer = tracing_subscriber::fmt::layer().with_writer(logs_file.with_max_level(Level::INFO)).with_ansi(false);

    let (logs_stderr, logs_stderr_guard) = tracing_appender::non_blocking(io::stderr());
    STDERR_GUARD.lock().expect("stderr_guard poisoned").replace(logs_stderr_guard);
    let (logs_stdout, logs_stdout_guard) = tracing_appender::non_blocking(io::stdout());
    STDOUT_GUARD.lock().expect("stdout_guard poisoned").replace(logs_stdout_guard);
    let logs_stdio_layer = tracing_subscriber::fmt::layer().with_writer(logs_stderr.with_max_level(Level::WARN).or_else(logs_stdout.with_max_level(Level::DEBUG))).with_ansi(true).boxed();

    drop(SUBSCRIBER_GUARD.lock().expect("subscriber_guard poisoned").take());

    let subscriber = tracing_subscriber::registry().with(logs_file_layer).with(logs_stdio_layer);
    tracing::subscriber::set_global_default(subscriber).expect("could not set global default subscriber");
}

/// Initialize the logging system before the configuration is loaded. Useful for errors during
/// config parsing.
pub fn pre_init() {
    let (logs_stderr, logs_stderr_guard) = tracing_appender::non_blocking(io::stderr());
    STDERR_GUARD.lock().expect("stderr_guard poisoned").replace(logs_stderr_guard);
    let (logs_stdout, logs_stdout_guard) = tracing_appender::non_blocking(io::stdout());
    STDOUT_GUARD.lock().expect("stdout_guard poisoned").replace(logs_stdout_guard);

    let layer = tracing_subscriber::fmt::layer().with_writer(logs_stderr.with_max_level(Level::WARN).or_else(logs_stdout.with_max_level(Level::DEBUG))).with_ansi(true).boxed();
    let subscriber = tracing_subscriber::registry().with(layer);
    SUBSCRIBER_GUARD.lock().expect("subscriber_guard poisoned").replace(tracing::subscriber::set_default(subscriber));
}

/// Flush the logs before the program exits.
pub fn flush() {
    drop(FILE_GUARD.lock().expect("file_guard poisoned").take());
    drop(STDERR_GUARD.lock().expect("stderr_guard poisoned").take());
    drop(STDOUT_GUARD.lock().expect("stdout_guard poisoned").take());
    drop(SUBSCRIBER_GUARD.lock().expect("subscriber_guard poisoned").take());
}
