#[warn(clippy::pedantic, clippy::cargo)]
mod config;
mod engine;
mod error;
mod file_map;
mod path_util;
mod world;

use std::fs;

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{error, info};

use crate::path_util::DisplayablePathBuf;
use crate::world::World;

#[derive(Parser)]
#[command(name = "didactic", about = "Simple typst SSG", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Increase logging verbosity
    #[arg(short, long, global = true)]
    verbose: bool,
    /// Show typst debug logging
    #[arg(long = "typst-verbose", global = true)]
    typst_verbose: bool
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
    let cli = Cli::parse();

    let log_level = if cli.typst_verbose {
        "debug"
    } else if cli.verbose {
        "info,didactic=debug"
    } else {
        "info"
    };
    env_logger::Builder::from_env(Env::default().default_filter_or(log_level)).init();

    match cli.command {
        Commands::Build { minify, dir } => {
            info!("Initializing compilation world");

            match World::new(dir.0, minify) {
                Ok(mut world) => {
                    if let Err(e) = world.build() {
                        error!("Build failed: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to initialize compilation world: {}", e);
                }
            }
        }
        Commands::Clean { dir } => {
            let output_path = dir.0.join("dist");
            if output_path.exists() {
                info!("Removing directory: {}", output_path.display());
                if let Err(e) = fs::remove_dir_all(&output_path) {
                    error!("Clean failed: {}", e);
                }
            } else {
                info!(
                    "Directory {} does not exist. Nothing to clean.",
                    output_path.display()
                );
            }
        }
    }
}
