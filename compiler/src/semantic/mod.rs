use crate::ast::*;
use crate::error::{AxError, AxResult};
use std::collections::HashSet;
use std::path::Path;

pub fn check(path: &Path) -> AxResult<()> {
    let src = std::fs::read_to_string(path)?;
    let prog = crate::parser::Parser::new(&src)?.parse_program()?;
    check_program(&prog)
}

pub fn check_program(prog: &Program) -> AxResult<()> {
    // 1. Unique names per category
    let mut screens = HashSet::new();
    let mut data = HashSet::new();
    let mut funcs = HashSet::new();
    for item in &prog.items {
        match item {
            Item::Screen(s) => {
                if !screens.insert(s.name.clone()) {
                    return Err(AxError::semantic(format!("duplikat ekranu '{}'", s.name), 0, 0));
                }
            }
            Item::Data(d) => {
                if !data.insert(d.name.clone()) {
                    return Err(AxError::semantic(format!("duplikat modelu '{}'", d.name), 0, 0));
                }
            }
            Item::Function(f) => {
                if !funcs.insert(f.name.clone()) {
                    return Err(AxError::semantic(format!("duplikat funkcji '{}'", f.name), 0, 0));
                }
            }
            _ => {}
        }
    }
    // 2. Screen navigation targets exist
    for item in &prog.items {
        if let Item::Screen(s) = item {
            for h in &s.handlers {
                check_stmts(&h.body, &screens)?;
            }
            for n in &s.body {
                check_ui_node(n, &screens)?;
            }
        }
    }
    // 3. Accessibility validation
    let issues = crate::a11y_validator::validate(prog);
    let criticals: Vec<_> = issues.iter().filter(|i| i.severity == crate::a11y_validator::Severity::Critical).collect();
    if !criticals.is_empty() {
        let mut msg = String::from("Błędy dostępności (critical):\n");
        for i in criticals {
            msg.push_str(&format!("  • [{}] {} — {}\n", i.rule, i.screen, i.message));
        }
        return Err(AxError::semantic(msg, 0, 0));
    }
    Ok(())
}

fn check_stmts(stmts: &[Stmt], screens: &HashSet<String>) -> AxResult<()> {
    for s in stmts {
        match s {
            Stmt::Go { target } => {
                let name = target.split('(').next().unwrap_or(target);
                if !screens.contains(name) {
                    return Err(AxError::semantic(format!("nawigacja do nieistniejącego ekranu '{}'", name), 0, 0));
                }
            }
            Stmt::If { then, els, .. } => {
                check_stmts(then, screens)?;
                check_stmts(els, screens)?;
            }
            Stmt::For { body, .. } => check_stmts(body, screens)?,
            Stmt::Call { on_success, on_error, .. } => {
                check_stmts(on_success, screens)?;
                check_stmts(on_error, screens)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_ui_node(n: &Node, screens: &HashSet<String>) -> AxResult<()> {
    if let Some(body) = &n.on_tap { check_stmts(body, screens)?; }
    for c in &n.children { check_ui_node(c, screens)?; }
    Ok(())
}
