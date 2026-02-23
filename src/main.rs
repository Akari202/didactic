#[warn(clippy::pedantic, clippy::cargo)]
mod build;
mod config;
mod file_map;
mod meta;
mod path_util;
#[cfg(test)]
mod test;

use std::fs;

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{error, info};

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
        /// Minify the html output
        #[arg(short, long)]
        minify: bool,
        /// The root directory to build
        #[arg(short, long, default_value_t = DisplayablePathBuf::from("./"))]
        dir: DisplayablePathBuf
    },

    /// Cleans the directory, ie deletes the dist folder
    Clean {
        /// The root directory of the build to clean
        #[arg(short, long, default_value_t = DisplayablePathBuf::from("./"))]
        dir: DisplayablePathBuf
    }
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { minify, dir } => {
            if let Err(e) = run_build(dir.0, minify) {
                error!("Build failed: {}", e);
            }
        }
        Commands::Clean { dir } => {
            let output_path = dir.0.join("dist");
            info!("Removing directory: {}", output_path.display());
            if let Err(e) = fs::remove_dir_all(&output_path) {
                error!("Failed: {}", e);
            }
        }
    }
}
