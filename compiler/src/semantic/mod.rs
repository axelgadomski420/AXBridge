use crate::ast::*;
use crate::error::{AxError, AxResult};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub fn check(path: &Path) -> AxResult<()> {
    let src = std::fs::read_to_string(path)?;
    let prog = crate::parser::Parser::new(&src)?.parse_program()?;
    check_program(&prog)
}

/// Analiza semantyczna: unikalność nazw, nawigacja, spójność modeli.
/// Reguły dostępności należą do `a11y::audit` na IR, więc nie ma tu dubli.
pub fn check_program(prog: &Program) -> AxResult<()> {
    let mut screens = HashSet::new();
    let mut data = HashSet::new();
    let mut funcs = HashSet::new();
    let mut stores = HashSet::new();
    let mut components = HashSet::new();

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
            Item::Store(s) => {
                if !stores.insert(s.name.clone()) {
                    return Err(AxError::semantic(format!("duplikat store '{}'", s.name), 0, 0));
                }
                if s.scope != "local" && s.scope != "cloud" {
                    return Err(AxError::semantic(
                        format!("store '{}' ma nieznany scope '{}' (local/cloud)", s.name, s.scope),
                        0,
                        0,
                    ));
                }
                if s.key.is_empty() {
                    return Err(AxError::semantic(
                        format!("store '{}' nie ma klucza wersjonowanego", s.name),
                        0,
                        0,
                    ));
                }
            }
            Item::Component(c) => {
                if !components.insert(c.name.clone()) {
                    return Err(AxError::semantic(format!("duplikat komponentu '{}'", c.name), 0, 0));
                }
            }
            _ => {}
        }
    }

    for item in &prog.items {
        match item {
            Item::Screen(s) => {
                for h in &s.handlers {
                    check_stmts(&h.body, &screens, &funcs)?;
                }
                for n in &s.body {
                    check_ui_node(n, &screens, &funcs)?;
                }
            }
            Item::Function(f) => check_stmts(&f.body, &screens, &funcs)?,
            Item::When(w) => check_stmts(&w.body, &screens, &funcs)?,
            Item::Timer(t) => check_stmts(&t.body, &screens, &funcs)?,
            _ => {}
        }
    }

    check_models(prog)?;
    Ok(())
}

/// Model bez pól albo z nieznanym typem rozjedzie się w generatorach.
fn check_models(prog: &Program) -> AxResult<()> {
    let models: HashMap<&str, &DataModel> = prog
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Data(d) => Some((d.name.as_str(), d)),
            _ => None,
        })
        .collect();

    for model in models.values() {
        if model.fields.is_empty() {
            return Err(AxError::semantic(format!("model '{}' nie ma pól", model.name), 0, 0));
        }
        let mut seen = HashSet::new();
        for field in &model.fields {
            if !seen.insert(field.name.clone()) {
                return Err(AxError::semantic(
                    format!("model '{}' ma zdublowane pole '{}'", model.name, field.name),
                    0,
                    0,
                ));
            }
            if let TypeRef::Named(name) = &field.ty {
                let known = ["Text", "Number", "Boolean", "Date", "Json", "List", "Map"];
                if !known.contains(&name.as_str()) && !models.contains_key(name.as_str()) {
                    return Err(AxError::semantic(
                        format!("pole '{}.{}' używa nieznanego typu '{}'", model.name, field.name, name),
                        0,
                        0,
                    ));
                }
            }
        }
    }
    Ok(())
}

fn check_stmts(stmts: &[Stmt], screens: &HashSet<String>, funcs: &HashSet<String>) -> AxResult<()> {
    for s in stmts {
        match s {
            Stmt::Go { target } => {
                let name = target.split('(').next().unwrap_or(target);
                if !screens.contains(name) {
                    return Err(AxError::semantic(
                        format!("nawigacja do nieistniejącego ekranu '{}'", name),
                        0,
                        0,
                    ));
                }
            }
            Stmt::CallFn { name, .. } => {
                if !funcs.contains(name) && !is_builtin(name) {
                    return Err(AxError::semantic(
                        format!("wywołanie nieistniejącej funkcji '{}'", name),
                        0,
                        0,
                    ));
                }
            }
            Stmt::If { then, els, .. } => {
                check_stmts(then, screens, funcs)?;
                check_stmts(els, screens, funcs)?;
            }
            Stmt::For { body, .. } => check_stmts(body, screens, funcs)?,
            Stmt::Call { on_success, on_error, .. } => {
                check_stmts(on_success, screens, funcs)?;
                check_stmts(on_error, screens, funcs)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "speak" | "vibrate" | "haptic" | "announce" | "translate" | "encrypt" | "decrypt" | "morseEncode" | "morseDecode"
    )
}

fn check_ui_node(n: &Node, screens: &HashSet<String>, funcs: &HashSet<String>) -> AxResult<()> {
    if let Some(body) = &n.on_tap {
        check_stmts(body, screens, funcs)?;
    }
    for c in &n.children {
        check_ui_node(c, screens, funcs)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Program {
        crate::parser::Parser::new(src).unwrap().parse_program().unwrap()
    }

    #[test]
    fn rejects_duplicate_screen_names() {
        let program = parse("app \"X\"\nversion \"1\"\nlanguage pl\nsupports web\nscreen Chat\n  title \"a\"\nscreen Chat\n  title \"b\"\n");
        assert!(check_program(&program).is_err());
    }

    #[test]
    fn rejects_navigation_to_missing_screen() {
        let program = parse("app \"X\"\nversion \"1\"\nlanguage pl\nsupports web\nscreen Chat\n  title \"a\"\n  button\n    label \"idź\"\n    when tapped\n      go Settings\n");
        assert!(check_program(&program).is_err());
    }

    #[test]
    fn accepts_valid_program() {
        let program = parse("app \"X\"\nversion \"1\"\nlanguage pl\nsupports web\nscreen Chat\n  title \"a\"\n  button\n    label \"idź\"\n    height 48\n    when tapped\n      go Chat\n");
        assert!(check_program(&program).is_ok());
    }

    #[test]
    fn rejects_empty_model() {
        let program = parse("app \"X\"\nversion \"1\"\nlanguage pl\nsupports web\ndata Empty\nscreen Chat\n  title \"a\"\n");
        assert!(check_program(&program).is_err());
    }

    #[test]
    fn rejects_unknown_field_type() {
        let program = parse("app \"X\"\nversion \"1\"\nlanguage pl\nsupports web\ndata M\n  a Brak\nscreen Chat\n  title \"a\"\n");
        assert!(check_program(&program).is_err());
    }
}
