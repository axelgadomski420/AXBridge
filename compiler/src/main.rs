//! AXCode Ultra Compiler (axc)
//! Production-grade compiler for AXBridge-class applications.
//! Supports: music, live translation, TikTak, personas, multi-provider AI,
//! Cloudflare Workers (D1/R2/KV/AI/Queue), iOS, Android, Web.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod lexer;
mod parser;
mod semantic;
mod axir;
mod optimizer;
mod codegen;
mod error;
mod runtime;

use error::AxResult;

#[derive(Parser)]
#[command(name = "axc")]
#[command(about = "AXCode Ultra Compiler – one file → every platform", long_about = None)]
#[command(version = "0.9.0-ultra")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile an .ax file
    Build {
        /// Path to the main .ax file
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Target platform
        #[arg(short, long, default_value = "web")]
        target: String,

        /// Output directory
        #[arg(short, long, default_value = "./dist")]
        out: PathBuf,

        /// Enable release optimizations
        #[arg(long)]
        release: bool,

        /// Verbose diagnostics
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run in development mode with hot reload
    Run {
        #[arg(value_name = "FILE")]
        file: PathBuf,

        #[arg(short, long, default_value = "web")]
        target: String,
    },

    /// Type-check and lint without generating code
    Check {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Format .ax source
    Format {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Show compiler + runtime diagnostics
    Doctor {},

    /// Generate Cloudflare Worker + D1 schema from .ax
    Workers {
        #[arg(value_name = "FILE")]
        file: PathBuf,

        #[arg(short, long, default_value = "./worker")]
        out: PathBuf,
    },
}

fn main() -> AxResult<()> {
    let cli = Cli::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    match cli.command {
        Commands::Build { file, target, out, release, verbose } => {
            info!("AXCode Ultra Compiler – building {} for {}", file.display(), target);
            codegen::build(&file, &target, &out, release, verbose)?;
        }
        Commands::Run { file, target } => {
            info!("Starting hot-reload dev server for {}", file.display());
            runtime::dev_server::start(&file, &target)?;
        }
        Commands::Check { file } => {
            semantic::check(&file)?;
            println!("✓ No errors found");
        }
        Commands::Format { file } => {
            // placeholder – real formatter later
            println!("Formatting {} …", file.display());
        }
        Commands::Doctor {} => {
            runtime::doctor::run();
        }
        Commands::Workers { file, out } => {
            info!("Generating Cloudflare Workers backend…");
            codegen::workers::generate(&file, &out)?;
        }
    }

    Ok(())
}
