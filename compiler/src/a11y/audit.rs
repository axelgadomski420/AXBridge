use super::validator::{A11yIssue, AttrValue, NodeView};

use crate::axir::build::{Ir, IrNode};

/// Spłaszcza węzeł IR do widoku walidatora, zachowując ścieżkę w drzewie.
pub fn node_view<'a>(node: &'a IrNode, path: Vec<usize>, buffer: &'a mut Vec<(String, AttrValue)>) -> NodeView<'a> {
    buffer.clear();
    for (key, value) in &node.props {
        let converted = match value {
            crate::ast::PropValue::Str(text) => AttrValue::Str(text.clone()),
            crate::ast::PropValue::Num(num) => AttrValue::Num(*num),
            crate::ast::PropValue::Ident(text) => AttrValue::Str(text.clone()),
            crate::ast::PropValue::Color(text) => AttrValue::Str(text.clone()),
            _ => AttrValue::Str(String::new()),
        };
        buffer.push((key.clone(), converted));
    }
    NodeView {
        kind: &node.kind,
        attrs: buffer,
        label: node.label.as_deref(),
        children: &[],
        path,
    }
}

/// Audyt całego programu na poziomie IR — uruchamiany przez `axc check`.
pub fn audit(ir: &Ir) -> Vec<A11yIssue> {
    let mut issues = Vec::new();
    for screen in &ir.screens {
        if screen.title.is_none() {
            issues.push(A11yIssue {
                rule: super::validator::A11yRule::ScreenTitle,
                severity: super::validator::Severity::Warning,
                screen: screen.name.clone(),
                node_path: vec![],
                message: format!("Ekran '{}' nie ma tytułu, czytnik nie ogłosi kontekstu", screen.name),
                remediation: "Dodaj `title \"…\"` do deklaracji ekranu.".into(),
                wcag: vec!["2.4.2".into()],
            });
        }
    }
    for screen in &ir.screens {
        walk(&screen.nodes, &screen.name, &mut issues);
    }
    issues
}

fn walk(nodes: &[IrNode], screen: &str, issues: &mut Vec<A11yIssue>) {
    for (index, node) in nodes.iter().enumerate() {
        let mut buffer = Vec::new();
        let view = node_view(node, vec![index], &mut buffer);
        let mut local = Vec::new();
        super::validator::validate(&view, &mut local);
        for mut issue in local {
            issue.screen = screen.to_string();
            issues.push(issue);
        }
        if node.kind == "video" && node.props.iter().all(|(k, _)| k != "captions") {
            issues.push(A11yIssue {
                rule: super::validator::A11yRule::CaptionTrack,
                severity: super::validator::Severity::Critical,
                screen: screen.to_string(),
                node_path: vec![index],
                message: "Wideo bez ścieżki napisów".into(),
                remediation: "Dodaj `captions` z transkrypcją — bez tego treść jest niedostępna.".into(),
                wcag: vec!["1.2.2".into()],
            });
        }
        if node.kind == "confirm" && node.channel.is_some() && !node.before_send {
            issues.push(A11yIssue {
                rule: super::validator::A11yRule::SignLanguageAlternative,
                severity: super::validator::Severity::Warning,
                screen: screen.to_string(),
                node_path: vec![index],
                message: format!("Kanał {:?} bez alternatywy potwierdzenia", node.channel),
                remediation: "Zapewnij równoważną reprezentację (napisy, alt, opis nastroju).".into(),
                wcag: vec!["1.1.1".into(), "1.2.1".into()],
            });
        }
        walk(&node.children, screen, issues);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axir::build::IrNode;

    fn leaf(kind: &str, props: Vec<(&str, crate::ast::PropValue)>, label: Option<&str>) -> IrNode {
        IrNode {
            kind: kind.into(),
            label: label.map(str::to_string),
            accessibility_label: None,
            alt: None,
            channel: None,
            before_send: false,
            aria_live: None,
            height: None,
            props: props.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            children: vec![],
            on_tap: vec![],
        }
    }

    fn ir_with(nodes: Vec<IrNode>, title: Option<&str>) -> Ir {
        Ir {
            app: crate::axir::build::IrApp {
                name: "X".into(), version: "1".into(), language: "pl".into(),
                platforms: vec![], offline_first: true, text_scale: 1.0,
                reduce_motion: false, high_contrast: false,
            },
            models: vec![], stores: vec![], globals: vec![], functions: vec![],
            screens: vec![crate::axir::build::IrScreen {
                name: "Chat".into(), title: title.map(str::to_string), states: vec![],
                nodes, handlers: vec![],
            }],
        }
    }

    #[test]
    fn audit_flags_missing_title() {
        let issues = audit(&ir_with(vec![leaf("text", vec![], Some("hej"))], None));
        assert!(issues.iter().any(|i| i.screen == "Chat"));
    }

    #[test]
    fn audit_requires_captions_for_video() {
        let issues = audit(&ir_with(vec![leaf("video", vec![("alt", crate::ast::PropValue::Str("film".into()))], None)], Some("Feed")));
        assert!(issues.iter().any(|i| i.rule == super::super::validator::A11yRule::CaptionTrack));
    }
}
