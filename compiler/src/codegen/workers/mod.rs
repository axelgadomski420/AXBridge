use crate::axir::build::Ir;
use crate::error::AxResult;
use std::path::Path;

/// Zgodność wsteczna: wywołanie z AST.
pub fn generate(file: &Path, out: &Path) -> AxResult<()> {
    let source = std::fs::read_to_string(file)?;
    let program = crate::parser::Parser::new(&source)?.parse_program()?;
    crate::semantic::check_program(&program)?;
    let ir = crate::axir::lower(&program);
    emit(&ir, out, false).map(|_| ())
}
