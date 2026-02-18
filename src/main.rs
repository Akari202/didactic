mod build;
mod config;
mod file_map;
mod meta;
mod path_util;

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::error;

use crate::build::run_build;
use crate::path_util::DisplayablePathBuf;

#[derive(Parser)]
#[command(name = "didactic", about = "Simple typst SSG", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    /// Build the website
    Build {
        /// The root directory to build
        #[arg(short, long, default_value_t = DisplayablePathBuf::from("./"))]
        dir: DisplayablePathBuf
    }
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { dir } => {
            if let Err(e) = run_build(dir.0) {
                error!("Build failed: {}", e);
            }
        }
    }
}
