use crate::ast::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity { Critical, Warning, Info }

#[derive(Debug)]
pub struct A11yIssue {
    pub screen: String,
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
}

pub fn validate(prog: &Program) -> Vec<A11yIssue> {
    let mut out = vec![];
    for item in &prog.items {
        if let Item::Screen(s) = item {
            validate_screen(s, &mut out);
        }
    }
    out
}

fn validate_screen(s: &Screen, out: &mut Vec<A11yIssue>) {
    if s.title.is_none() {
        out.push(A11yIssue {
            screen: s.name.clone(),
            rule: "AX-4.1.2",
            severity: Severity::Warning,
            message: "Ekran bez title — czytniki ekranu mogą nie ogłosić kontekstu".into(),
        });
    }
    for n in &s.body { validate_node(&s.name, n, out); }
}

fn validate_node(screen: &str, n: &Node, out: &mut Vec<A11yIssue>) {
    let interactive = matches!(n.kind.as_str(), "button" | "toggle" | "slider" | "field" | "checkbox");
    if interactive {
        let has_label = n.attrs.iter().any(|(k, v)| {
            k == "label" || k == "accessibility" || matches!(v, PropValue::Str(_))
        });
        if !has_label {
            out.push(A11yIssue {
                screen: screen.into(),
                rule: "AX-4.1.2",
                severity: Severity::Critical,
                message: format!("Interaktywny '{}' bez etykiety (label/accessibility)", n.kind),
            });
        }
    }
    // Check min size for buttons
    if n.kind == "button" {
        let h = n.attrs.iter().find(|(k, _)| k == "height").map(|(_, v)| v);
        let w = n.attrs.iter().find(|(k, _)| k == "width").map(|(_, v)| v);
        for (name, val) in [("height", h), ("width", w)] {
            if let Some(PropValue::Num(v)) = val {
                if *v > 0.0 && *v < 44.0 {
                    out.push(A11yIssue {
                        screen: screen.into(),
                        rule: "AX-2.5.8",
                        severity: Severity::Critical,
                        message: format!("Button {} = {} (< 44pt)", name, v),
                    });
                }
            }
        }
    }
    for c in &n.children { validate_node(screen, c, out); }
}
