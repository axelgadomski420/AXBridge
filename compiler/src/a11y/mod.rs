pub mod adaptation;
pub mod validator;

pub use adaptation::{plan_for, AdaptationPlan, Channel, Direction};
pub use validator::{summarize, validate, A11yIssue, A11yRule, AttrValue, NodeView, Severity};

/// Kanoniczny raport dostępności dla diagnostyki `axc check`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccessibilityReport {
    pub critical: usize,
    pub warning: usize,
    pub info: usize,
    pub issues: Vec<A11yIssue>,
}

impl AccessibilityReport {
    pub fn from_issues(issues: Vec<A11yIssue>) -> Self {
        let (critical, warning, info) = summarize(&issues);
        Self { critical, warning, info, issues }
    }

    pub fn is_blocking(&self) -> bool {
        self.critical > 0
    }

    pub fn render(&self) -> String {
        let mut out = format!(
            "Accessibility: {} critical, {} warning, {} info\n",
            self.critical, self.warning, self.info
        );
        for issue in &self.issues {
            out.push_str(&format!(
                "  • [{:?}] {:?} at {:?} — {}\n    fix: {}\n    wcag: {}\n",
                issue.severity,
                issue.rule,
                issue.node_path,
                issue.message,
                issue.remediation,
                issue.wcag.join(", ")
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocking_only_on_critical() {
        let report = AccessibilityReport { critical: 1, warning: 4, info: 0, issues: vec![] };
        assert!(report.is_blocking());
        let clean = AccessibilityReport { critical: 0, warning: 9, info: 2, issues: vec![] };
        assert!(!clean.is_blocking());
    }

    #[test]
    fn plan_is_exposed_at_module_root() {
        assert_eq!(plan_for("blind").persona, "blind");
    }
}
