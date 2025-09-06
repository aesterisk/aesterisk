use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Cortex", about = "Developer experience, reimagined.", long_about = None, long_version = concat!("v", env!("CARGO_PKG_VERSION"), "\n\nMade with <3 by Valter Sporrenstrand\n(https://github.com/yolocat-dev)"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a new Cortex project
    Init,
    /// Generate a new specification
    New {
        /// The name of the specification to create
        name: String,

        /// Is this an API specification
        #[arg(short, long, default_value_t = false)]
        api: bool,
    },
    /// Synchronize the Cortex project with the latest specifications
    Sync,
}
