//! AXCode Ultra Compiler — CLI i eksport biblioteczny.

pub mod a11y;
pub mod ai;
pub mod ast;
pub mod axir;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod message;
pub mod optimizer;
pub mod parser;
pub mod runtime;
pub mod semantic;

pub use axir::{Ir, IrNode, IrScreen};
pub use error::{AxError, AxResult};

pub fn compile(source: &str) -> AxResult<(Ir, Vec<a11y::A11yIssue>)> {
    let program = parser::Parser::new(source)?.parse_program()?;
    semantic::check_program(&program)?;
    let ir = axir::lower(&program);
    let issues = a11y::audit::audit(&ir);
    Ok((ir, issues))
}

pub fn compile_file(path: &std::path::Path) -> AxResult<(Ir, Vec<a11y::A11yIssue>)> {
    let source = std::fs::read_to_string(path)?;
    compile(&source)
}

pub fn persona_plan(persona: &str) -> a11y::AdaptationPlan {
    a11y::plan_for(persona)
}

pub fn channel_matrix() -> Vec<message::CapabilityRow> {
    message::capability_matrix()
}
