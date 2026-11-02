pub mod swift;
pub mod kotlin;
pub mod typescript;
pub mod workers;

use crate::ast::Program;
use crate::axir;
use crate::error::AxResult;
use crate::optimizer;
use std::path::Path;

pub fn build(file: &Path, target: &str, out: &Path, release: bool, verbose: bool) -> AxResult<()> {
    let src = std::fs::read_to_string(file)?;
    let mut prog = crate::parser::Parser::new(&src)?.parse_program()?;
    crate::semantic::check_program(&prog)?;
    if release { optimizer::optimize(&mut prog); }
    let ir = axir::lower(&prog);
    std::fs::create_dir_all(out)?;
    match target {
        "ios" | "swift" => swift::emit(&ir, out, verbose)?,
        "android" | "kotlin" => kotlin::emit(&ir, out, verbose)?,
        "web" | "ts" | "typescript" => typescript::emit(&ir, out, verbose)?,
        "worker" | "workers" | "cloudflare" => workers::emit(&ir, out, verbose)?,
        other => return Err(crate::error::AxError::codegen(format!("nieznany target: {}", other))),
    }
    Ok(())
}
