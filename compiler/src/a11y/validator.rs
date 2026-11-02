use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum A11yRule {
    InteractiveLabel,
    ScreenTitle,
    TouchTargetSize,
    LiveRegion,
    FocusOrder,
    ReducedMotion,
    ContrastToken,
    ChannelRedundancy,
    ConfirmBeforeSend,
    TextAlternative,
    CaptionTrack,
    SignLanguageAlternative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A11yIssue {
    pub rule: A11yRule,
    pub severity: Severity,
    pub screen: String,
    pub node_path: Vec<usize>,
    pub message: String,
    pub remediation: String,
    pub wcag: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NodeView<'a> {
    pub kind: &'a str,
    pub attrs: &'a [(String, AttrValue)],
    pub label: Option<&'a str>,
    pub children: &'a [NodeView<'a>],
    pub path: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    Str(String),
    Num(f64),
    Bool(bool),
}

const INTERACTIVE: &[&str] = &["button", "toggle", "slider", "field", "checkbox", "picker", "switch"];
const CHANNELS: &[&str] = &["text", "speech", "morse", "sign", "music", "vibration", "visual", "haptic"];
const MIN_TARGET: f64 = 44.0;

pub fn validate(node: &NodeView<'_>, issues: &mut Vec<A11yIssue>) {
    let interactive = INTERACTIVE.contains(&node.kind);
    let has_label = node.label.is_some()
        || node.attrs.iter().any(|(k, _)| k == "label" || k == "accessibility" || k == "alt");
    if interactive && !has_label {
        issues.push(A11yIssue {
            rule: A11yRule::InteractiveLabel,
            severity: Severity::Critical,
            screen: String::new(),
            node_path: node.path.clone(),
            message: format!("Interactive `{}` has no accessible label", node.kind),
            remediation: "Add `accessibility label \"…\"` or a visible text child.".into(),
            wcag: vec!["4.1.2".into(), "2.4.6".into()],
        });
    }
    if (node.kind == "image" || node.kind == "video") && !has_label {
        issues.push(A11yIssue {
            rule: A11yRule::TextAlternative,
            severity: Severity::Critical,
            screen: String::new(),
            node_path: node.path.clone(),
            message: format!("`{}` has no text alternative", node.kind),
            remediation: "Add `alt \"…\"` describing the content, not the file name.".into(),
            wcag: vec!["1.1.1".into()],
        });
    }
    if interactive {
        for key in ["height", "width"] {
            if let Some((_, AttrValue::Num(v))) = node.attrs.iter().find(|(k, _)| k == key) {
                if *v > 0.0 && *v < MIN_TARGET {
                    issues.push(A11yIssue {
                        rule: A11yRule::TouchTargetSize,
                        severity: Severity::Critical,
                        screen: String::new(),
                        node_path: node.path.clone(),
                        message: format!("`{}` target is {:.0}pt, below {:.0}pt", key, v, MIN_TARGET),
                        remediation: "Increase the target or add padding so the hit area reaches 44pt.".into(),
                        wcag: vec!["2.5.8".into(), "2.5.5".into()],
                    });
                }
            }
        }
    }
    let motion = node
        .attrs
        .iter()
        .any(|(k, v)| k == "animate" || (k == "motion" && *v == AttrValue::Bool(true)));
    if motion {
        let guarded = node.attrs.iter().any(|(k, _)| k == "respectsReduceMotion" || k == "guarded");
        if !guarded {
            issues.push(A11yIssue {
                rule: A11yRule::ReducedMotion,
                severity: Severity::Warning,
                screen: String::new(),
                node_path: node.path.clone(),
                message: "Animation without a reduced-motion guard".into(),
                remediation: "Add `respects reduceMotion` so motion-sensitive users are not exposed.".into(),
                wcag: vec!["2.3.3".into()],
            });
        }
    }
    let announces = node.attrs.iter().any(|(k, _)| k == "aria-live" || k == "liveRegion");
    if (node.kind == "list" || node.kind == "stream") && !announces {
        issues.push(A11yIssue {
            rule: A11yRule::LiveRegion,
            severity: Severity::Warning,
            screen: String::new(),
            node_path: node.path.clone(),
            message: format!("`{}` receives dynamic content without a live region", node.kind),
            remediation: "Mark the stream `aria-live polite` so screen readers announce new items.".into(),
            wcag: vec!["4.1.3".into()],
        });
    }
    let mut seen: HashSet<&str> = HashSet::new();
    for (key, value) in node.attrs {
        if key == "channel" {
            if let AttrValue::Str(channel) = value {
                if !CHANNELS.contains(&channel.as_str()) {
                    issues.push(A11yIssue {
                        rule: A11yRule::ChannelRedundancy,
                        severity: Severity::Warning,
                        screen: String::new(),
                        node_path: node.path.clone(),
                        message: format!("Unknown channel `{}`", channel),
                        remediation: "Use one of the declared accessibility channels.".into(),
                        wcag: vec!["1.1.1".into()],
                    });
                }
                if !seen.insert(channel.as_str()) {
                    issues.push(A11yIssue {
                        rule: A11yRule::ChannelRedundancy,
                        severity: Severity::Info,
                        screen: String::new(),
                        node_path: node.path.clone(),
                        message: format!("Channel `{}` declared twice", channel),
                        remediation: "Declare each channel once per node.".into(),
                        wcag: vec!["1.1.1".into()],
                    });
                }
            }
        }
    }
    if node.kind == "confirm" {
        let explicit = node
            .attrs
            .iter()
            .any(|(k, v)| k == "beforeSend" && *v == AttrValue::Bool(true));
        if !explicit {
            issues.push(A11yIssue {
                rule: A11yRule::ConfirmBeforeSend,
                severity: Severity::Critical,
                screen: String::new(),
                node_path: node.path.clone(),
                message: "AI-composed content can be sent without human confirmation".into(),
                remediation: "Add `confirm beforeSend`; automatic sending must be opt-in per channel.".into(),
                wcag: vec!["3.3.4".into()],
            });
        }
    }
    for (index, child) in node.children.iter().enumerate() {
        let mut next = child.clone();
        next.path = {
            let mut path = node.path.clone();
            path.push(index);
            path
        };
        validate(&next, issues);
    }
}

pub fn summarize(issues: &[A11yIssue]) -> (usize, usize, usize) {
    let critical = issues.iter().filter(|i| i.severity == Severity::Critical).count();
    let warning = issues.iter().filter(|i| i.severity == Severity::Warning).count();
    let info = issues.iter().filter(|i| i.severity == Severity::Info).count();
    (critical, warning, info)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node<'a>(kind: &'a str, attrs: &'a [(String, AttrValue)], children: &'a [NodeView<'a>]) -> NodeView<'a> {
        NodeView { kind, attrs, label: None, children, path: vec![] }
    }

    #[test]
    fn flags_unlabelled_button() {
        let attrs: Vec<(String, AttrValue)> = vec![];
        let target = node("button", &attrs, &[]);
        let mut issues = vec![];
        validate(&target, &mut issues);
        assert!(issues.iter().any(|i| i.rule == A11yRule::InteractiveLabel));
    }

    #[test]
    fn accepts_labelled_button_with_target() {
        let attrs: Vec<(String, AttrValue)> = vec![
            ("label".into(), AttrValue::Str("Wyślij".into())),
            ("height".into(), AttrValue::Num(52.0)),
        ];
        let target = node("button", &attrs, &[]);
        let mut issues = vec![];
        validate(&target, &mut issues);
        assert!(issues.is_empty());
    }

    #[test]
    fn flags_small_target() {
        let attrs: Vec<(String, AttrValue)> = vec![
            ("label".into(), AttrValue::Str("OK".into())),
            ("height".into(), AttrValue::Num(32.0)),
        ];
        let target = node("button", &attrs, &[]);
        let mut issues = vec![];
        validate(&target, &mut issues);
        assert!(issues.iter().any(|i| i.rule == A11yRule::TouchTargetSize));
    }

    #[test]
    fn requires_confirmation_for_ai_channels() {
        let attrs: Vec<(String, AttrValue)> = vec![("channel".into(), AttrValue::Str("sign".into()))];
        let target = node("confirm", &attrs, &[]);
        let mut issues = vec![];
        validate(&target, &mut issues);
        assert!(issues.iter().any(|i| i.rule == A11yRule::ConfirmBeforeSend));
    }

    #[test]
    fn summary_counts_by_severity() {
        let attrs: Vec<(String, AttrValue)> = vec![];
        let target = node("button", &attrs, &[]);
        let mut issues = vec![];
        validate(&target, &mut issues);
        let (critical, _, _) = summarize(&issues);
        assert!(critical >= 1);
    }
}
