use crate::axir::Ir;
use crate::error::AxResult;
use std::path::Path;

pub mod kotlin;
pub mod swift;
pub mod typescript;
pub mod web;
pub mod workers;

/// Wynik kompilacji: ile artefaktów powstało i czego generator nie obsłużył.
#[derive(Debug, Default, Clone)]
pub struct BuildOutcome {
    pub artefacts: usize,
    pub unsupported: Vec<String>,
}

pub fn build_ir(ir: &Ir, target: &str, out: &Path, _release: bool, verbose: bool) -> AxResult<BuildOutcome> {
    std::fs::create_dir_all(out)?;
    let mut outcome = BuildOutcome::default();
    match target {
        "web" | "ts" | "typescript" => {
            let report = web::emit(ir, out, verbose)?;
            outcome.artefacts = 3;
            outcome.unsupported = report.unsupported;
        }
        "ios" | "swift" => {
            let report = swift::emit(ir, out, verbose)?;
            outcome.artefacts = report.files;
            outcome.unsupported = report.unsupported;
        }
        "android" | "kotlin" => {
            let report = kotlin::emit(ir, out, verbose)?;
            outcome.artefacts = report.files;
            outcome.unsupported = report.unsupported;
        }
        "worker" | "workers" | "cloudflare" => {
            let report = workers::emit(ir, out, verbose)?;
            outcome.artefacts = report.files;
            outcome.unsupported = report.unsupported;
        }
        other => return Err(crate::error::AxError::codegen(format!("nieznany target: {}", other))),
    }
    Ok(outcome)
}

/// Zgodność wsteczna dla starszych wywołań z AST.
pub fn build(file: &Path, target: &str, out: &Path, release: bool, verbose: bool) -> AxResult<()> {
    let source = std::fs::read_to_string(file)?;
    let program = crate::parser::Parser::new(&source)?.parse_program()?;
    crate::semantic::check_program(&program)?;
    let ir = crate::axir::lower(&program);
    build_ir(&ir, target, out, release, verbose).map(|_| ())
}
