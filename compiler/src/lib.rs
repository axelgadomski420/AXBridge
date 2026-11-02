//! AXCode Ultra Compiler — CLI i eksport biblioteczny.

pub use axir::{Ir, IrScreen, IrNode};
pub use error::{AxError, AxResult};

pub fn compile(source: &str) -> AxResult<(Ir, Vec<a11y::A11yIssue>)> {
    let program = parser::Parser::new(source)?.parse_program()?;
    semantic::check_program(&program)?;
    let ir = axir::lower(&program);
    let issues = a11y::audit::audit(&ir);
    Ok((ir, issues))
}

pub fn persona_plan(persona: &str) -> a11y::AdaptationPlan {
    a11y::plan_for(persona)
}

pub fn channel_matrix() -> Vec<message::CapabilityRow> {
    message::capability_matrix()
}
