//! Semantic analysis, type checking, persona resolution, provider validation

use crate::error::{codes, AxError, AxResult};
use crate::parser::AstNode;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct SemanticContext {
    pub screens: HashMap<String, ()>,
    pub components: HashMap<String, ()>,
    pub stores: HashMap<String, String>,
    pub functions: HashMap<String, ()>,
    pub providers: HashMap<String, bool>, // id → enabled
    pub persona: Option<String>,
    pub has_admin_guard: bool,
}

pub fn check(path: &Path) -> AxResult<()> {
    let source = std::fs::read_to_string(path)?;
    let ast = crate::parser::parse_file(&source, path.to_str().unwrap_or("app.ax"))?;
    let mut ctx = SemanticContext::default();
    analyze(&ast, &mut ctx)?;

    // Basic validations required by AXBridge Ultra
    if !ctx.providers.contains_key("puter") {
        // warning only in real impl
    }
    Ok(())
}

fn analyze(node: &AstNode, ctx: &mut SemanticContext) -> AxResult<()> {
    match node {
        AstNode::App { body, .. } => {
            for child in body {
                analyze(child, ctx)?;
            }
        }
        AstNode::Screen { name, body, .. } => {
            ctx.screens.insert(name.clone(), ());
            for child in body {
                analyze(child, ctx)?;
            }
        }
        AstNode::Component { name, body, .. } => {
            ctx.components.insert(name.clone(), ());
            for child in body {
                analyze(child, ctx)?;
            }
        }
        AstNode::Function { name, body, .. } => {
            ctx.functions.insert(name.clone(), ());
            for child in body {
                analyze(child, ctx)?;
            }
        }
        AstNode::ProviderDecl { id, .. } => {
            ctx.providers.insert(id.clone(), true);
        }
        AstNode::PersonaSet { value } => {
            ctx.persona = Some(value.clone());
        }
        _ => {}
    }
    Ok(())
}

pub fn validate_provider_usage(provider: &str, ctx: &SemanticContext) -> AxResult<()> {
    match ctx.providers.get(provider) {
        Some(true) => Ok(()),
        Some(false) => Err(AxError::Semantic {
            code: codes::PROVIDER_DISABLED,
            message_pl: format!("Provider „{}” jest wyłączony.", provider),
            message_en: format!("Provider “{}” is disabled.", provider),
            span: 0..0,
            file: "app.ax".into(),
            suggestion: Some("Włącz go w Ustawienia → Dostawcy API".into()),
        }),
        None => Err(AxError::Semantic {
            code: codes::PROVIDER_DISABLED,
            message_pl: format!("Nieznany provider „{}”.", provider),
            message_en: format!("Unknown provider “{}”.", provider),
            span: 0..0,
            file: "app.ax".into(),
            suggestion: Some("Dodaj go w ProvidersSettings".into()),
        }),
    }
}
