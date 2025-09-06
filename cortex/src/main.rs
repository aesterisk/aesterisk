use clap::Parser;

use crate::cli::{Cli, Commands};

mod cli;
mod commands;
mod config;
mod spec;
mod utils;

fn main() -> anyhow::Result<()> {
    let command = Cli::parse().command;

    match command {
        Commands::Init => commands::init::run(),
        Commands::New { name, api } => commands::new::run(name, api),
        Commands::Sync => commands::sync::run()?,
    }

    Ok(())
}
