//! AXCode Ultra Compiler (axc) — produkcyjny kompilator dla aplikacji klasy AXBridge.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod a11y;
mod ai;
mod ast;
mod axir;
mod codegen;
mod error;
mod lexer;
mod message;
mod optimizer;
mod parser;
mod runtime;
mod semantic;

use error::AxResult;

#[derive(Parser)]
#[command(name = "axc")]
#[command(about = "AXCode Ultra Compiler – one file → every platform", long_about = None)]
#[command(version = "2.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile an .ax file
    Build {
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Target platform: web | react | ios | android | workers
        #[arg(short, long, default_value = "web")]
        target: String,

        #[arg(short, long, default_value = "./dist")]
        out: PathBuf,

        #[arg(long)]
        release: bool,

        #[arg(short, long)]
        verbose: bool,

        /// Continue the build despite critical accessibility findings
        #[arg(long)]
        allow_a11y_violations: bool,
    },

    /// Run in development mode with rebuild on change
    Run {
        #[arg(value_name = "FILE")]
        file: PathBuf,

        #[arg(short, long, default_value = "web")]
        target: String,
    },

    /// Type-check, lint and audit accessibility without generating code
    Check {
        #[arg(value_name = "FILE")]
        file: PathBuf,

        #[arg(long)]
        json: bool,
    },

    /// Print the accessibility report for an .ax file
    Audit {
        #[arg(value_name = "FILE")]
        file: PathBuf,

        #[arg(long)]
        json: bool,
    },

    /// Print the adaptation plan for a persona (deaf, blind, deafBlind, motor, …)
    Persona {
        #[arg(value_name = "PERSONA")]
        persona: String,

        #[arg(long)]
        json: bool,
    },

    /// Print the multi-channel capability matrix
    Channels {
        #[arg(long)]
        json: bool,
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

fn load(file: &Path) -> AxResult<axir::Ir> {
    let source = std::fs::read_to_string(file)?;
    let program = parser::Parser::new(&source)?.parse_program()?;
    semantic::check_program(&program)?;
    Ok(axir::lower(&program))
}

fn main() -> AxResult<()> {
    let cli = Cli::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    match cli.command {
        Commands::Build { file, target, out, release, verbose, allow_a11y_violations } => {
            info!("AXCode — building {} for {}", file.display(), target);
            let ir = load(&file)?;
            let report = a11y::AccessibilityReport::from_issues(a11y::audit::audit(&ir));
            if report.is_blocking() && !allow_a11y_violations {
                eprint!("{}", report.render());
                return Err(error::AxError::semantic(
                    "Build zatrzymany: krytyczne błędy dostępności".into(),
                    0,
                    0,
                ));
            }
            let outcome = codegen::build_ir(&ir, &target, &out, release, verbose)?;
            println!("Accessibility: {} critical, {} warning, {} info", report.critical, report.warning, report.info);
            if !outcome.unsupported.is_empty() {
                println!("Unsupported nodes: {}", outcome.unsupported.join(", "));
            }
            println!("✓ {} artefaktów zapisano w {}", outcome.artefacts, out.display());
        }
        Commands::Run { file, target } => {
            info!("Starting dev watcher for {}", file.display());
            runtime::dev_server::start(&file, &target)?;
        }
        Commands::Check { file, json } => {
            semantic::check(&file)?;
            let ir = load(&file)?;
            let report = a11y::AccessibilityReport::from_issues(a11y::audit::audit(&ir));
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into()));
            } else {
                print!("{}", report.render());
            }
            if report.is_blocking() {
                return Err(error::AxError::semantic("Krytyczne błędy dostępności".into(), 0, 0));
            }
            println!("✓ Bez błędów krytycznych");
        }
        Commands::Audit { file, json } => {
            let ir = load(&file)?;
            let report = a11y::AccessibilityReport::from_issues(a11y::audit::audit(&ir));
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into()));
            } else {
                print!("{}", report.render());
            }
        }
        Commands::Persona { persona, json } => {
            let plan = a11y::plan_for(&persona);
            if json {
                println!("{}", serde_json::to_string_pretty(&plan).unwrap_or_else(|_| "{}".into()));
            } else {
                println!("Persona: {}", plan.persona);
                println!("  inbound:  {}", plan.inbound.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", "));
                println!("  outbound: {}", plan.outbound.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", "));
                println!(
                    "  large targets: {}, reduce motion: {}, max actions: {}, announce: {}",
                    plan.large_targets, plan.reduce_motion, plan.max_actions, plan.announce_changes
                );
                for channel in plan.inbound.iter().chain(plan.outbound.iter()) {
                    let fallback = channel.fallback.clone().unwrap_or_else(|| "-".into());
                    println!(
                        "    {} confirm={} offline={} fallback={}",
                        channel.id, channel.requires_confirmation, channel.offline_capable, fallback
                    );
                }
            }
        }
        Commands::Channels { json } => {
            let matrix = message::capability_matrix();
            if json {
                println!("{}", serde_json::to_string_pretty(&matrix).unwrap_or_else(|_| "{}".into()));
            } else {
                for row in &matrix {
                    println!(
                        "{:>10}  {:>14}  confirms={:<5} offline={:<5} redundant={}",
                        row.channel, row.kind, row.requires_confirmation, row.offline_capable, row.redundant_representation
                    );
                }
            }
        }
        Commands::Format { file } => {
            println!("Formatting {} …", file.display());
        }
        Commands::Doctor {} => {
            runtime::doctor::run();
        }
        Commands::Workers { file, out } => {
            info!("Generating Cloudflare Workers backend…");
            let ir = load(&file)?;
            let report = codegen::workers::emit(&ir, &out, false)?;
            println!("✓ {} plików zapisano w {}", report.files, out.display());
        }
    }

    Ok(())
}
