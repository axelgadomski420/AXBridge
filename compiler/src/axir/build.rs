use crate::ast::{Expr, Item, Node, Program, PropValue, Screen, Stmt, TypeRef};

/// Model przejściowy pomiędzy AST a generatorami. Generatory nie czytają AST —
/// czytają wyłącznie ten opis, dzięki czemu semantyka jest identyczna na każdej platformie.
#[derive(Debug, Clone, PartialEq)]
pub struct Ir {
    pub app: IrApp,
    pub models: Vec<IrModel>,
    pub stores: Vec<IrStore>,
    pub globals: Vec<IrGlobal>,
    pub functions: Vec<IrFunction>,
    pub screens: Vec<IrScreen>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrApp {
    pub name: String,
    pub version: String,
    pub language: String,
    pub platforms: Vec<String>,
    pub offline_first: bool,
    pub text_scale: f64,
    pub reduce_motion: bool,
    pub high_contrast: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrModel {
    pub name: String,
    pub fields: Vec<IrField>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrField {
    pub name: String,
    pub ty: String,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrStore {
    pub name: String,
    pub kind: String,
    pub scope: String,
    pub ty: String,
    pub key: String,
    pub encrypted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrGlobal {
    pub name: String,
    pub ty: String,
    pub default: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub ret: Option<String>,
    pub body: Vec<IrStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrScreen {
    pub name: String,
    pub title: Option<String>,
    pub states: Vec<(String, String, String)>,
    pub nodes: Vec<IrNode>,
    pub handlers: Vec<IrHandler>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrHandler {
    pub event: String,
    pub body: Vec<IrStmt>,
}

/// Węzeł UI z rozwiązanymi atrybutami dostępności. Każdy węzeł interaktywny
/// ma etykietę albo `None`, co walidator zamienia w błąd krytyczny.
#[derive(Debug, Clone, PartialEq)]
pub struct IrNode {
    pub kind: String,
    pub label: Option<String>,
    pub accessibility_label: Option<String>,
    pub alt: Option<String>,
    pub channel: Option<String>,
    pub before_send: bool,
    pub aria_live: Option<String>,
    pub height: Option<f64>,
    pub props: Vec<(String, PropValue)>,
    pub children: Vec<IrNode>,
    pub on_tap: Vec<IrStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrStmt {
    Set { target: String, value: Expr },
    Save { target: String },
    Load { target: String },
    Go { screen: String },
    GoBack,
    Toast(Expr),
    Alert(Expr),
    Api { path: String, method: String, on_success: Vec<IrStmt>, on_error: Vec<IrStmt> },
    CallFn { name: String, args: Vec<Expr> },
    If { cond: Expr, then: Vec<IrStmt>, els: Vec<IrStmt> },
    ForEach { var: String, iter: Expr, body: Vec<IrStmt> },
    Add { target: String, value: Expr },
    Remove { target: String, value: Expr },
    Return(Option<Expr>),
}

pub fn lower(program: &Program) -> Ir {
    let mut models = Vec::new();
    let mut stores = Vec::new();
    let mut globals = Vec::new();
    let mut functions = Vec::new();
    let mut screens = Vec::new();

    for item in &program.items {
        match item {
            Item::Data(model) => models.push(IrModel {
                name: model.name.clone(),
                fields: model
                    .fields
                    .iter()
                    .map(|field| IrField {
                        name: field.name.clone(),
                        ty: type_name(&field.ty).0,
                        optional: type_name(&field.ty).1,
                    })
                    .collect(),
            }),
            Item::Store(store) => stores.push(IrStore {
                name: store.name.clone(),
                kind: match store.kind {
                    crate::ast::StoreKind::Single => "single".into(),
                    crate::ast::StoreKind::List => "list".into(),
                    crate::ast::StoreKind::Map => "map".into(),
                },
                scope: store.scope.clone(),
                ty: store.ty.clone(),
                key: store.key.clone(),
                encrypted: store.encrypted,
            }),
            Item::GlobalState(vars) => globals.extend(vars.iter().map(|var| IrGlobal {
                name: var.name.clone(),
                ty: type_name(&var.ty).0,
                default: var.default.clone(),
            })),
            Item::Function(function) => functions.push(IrFunction {
                name: function.name.clone(),
                params: function.params.iter().map(|p| (p.name.clone(), type_name(&p.ty).0)).collect(),
                ret: function.ret.as_ref().map(|t| type_name(t).0),
                body: lower_stmts(&function.body),
            }),
            Item::Screen(screen) => screens.push(lower_screen(screen)),
            _ => {}
        }
    }

    let text_scale = screens
        .iter()
        .flat_map(|screen| screen.states.iter())
        .find(|(name, _, _)| name == "textScale")
        .and_then(|(_, _, value)| value.parse::<f64>().ok())
        .unwrap_or(1.0);

    Ir {
        app: IrApp {
            name: program.app.name.clone(),
            version: program.app.version.clone(),
            language: program.app.language.clone(),
            platforms: program.app.supports.clone(),
            offline_first: program.app.offline_first,
            text_scale: if text_scale > 0.0 { text_scale } else { 1.0 },
            reduce_motion: false,
            high_contrast: false,
        },
        models,
        stores,
        globals,
        functions,
        screens,
    }
}

fn lower_screen(screen: &Screen) -> IrScreen {
    IrScreen {
        name: screen.name.clone(),
        title: screen.title.clone(),
        states: screen
            .states
            .iter()
            .map(|state| (state.name.clone(), type_name(&state.ty).0, state.default.clone()))
            .collect(),
        nodes: screen.body.iter().map(lower_node).collect(),
        handlers: screen
            .handlers
            .iter()
            .map(|handler| IrHandler { event: handler.event.clone(), body: lower_stmts(&handler.body) })
            .collect(),
    }
}

fn lower_node(node: &Node) -> IrNode {
    let mut label = None;
    let mut accessibility_label = None;
    let mut alt = None;
    let mut channel = None;
    let mut before_send = false;
    let mut aria_live = None;
    let mut height = None;

    for (key, value) in &node.attrs {
        match (key.as_str(), value) {
            ("label", PropValue::Str(text)) => label = Some(text.clone()),
            ("accessibility", PropValue::Str(text)) => accessibility_label = Some(text.clone()),
            ("alt", PropValue::Str(text)) => alt = Some(text.clone()),
            ("channel", PropValue::Str(text)) => channel = Some(text.clone()),
            ("beforeSend", PropValue::Ident(text)) | ("beforeSend", PropValue::Str(text)) => {
                before_send = text == "true"
            }
            ("aria-live", PropValue::Str(text)) => aria_live = Some(text.clone()),
            ("height", PropValue::Num(value)) => height = Some(*value),
            _ => {}
        }
    }

    IrNode {
        kind: node.kind.clone(),
        label,
        accessibility_label,
        alt,
        channel,
        before_send,
        aria_live,
        height,
        props: node.attrs.clone(),
        children: node.children.iter().map(lower_node).collect(),
        on_tap: node.on_tap.as_ref().map(|body| lower_stmts(body)).unwrap_or_default(),
    }
}

fn lower_stmts(stmts: &[Stmt]) -> Vec<IrStmt> {
    stmts
        .iter()
        .map(|stmt| match stmt {
            Stmt::Set { target, value } => IrStmt::Set { target: target.clone(), value: value.clone() },
            Stmt::Save { target } => IrStmt::Save { target: target.clone() },
            Stmt::Load { target } => IrStmt::Load { target: target.clone() },
            Stmt::Go { target } => IrStmt::Go { screen: target.split('(').next().unwrap_or(target).to_string() },
            Stmt::GoBack => IrStmt::GoBack,
            Stmt::ShowToast(expr) => IrStmt::Toast(expr.clone()),
            Stmt::ShowAlert(expr) => IrStmt::Alert(expr.clone()),
            Stmt::Call { path, method, on_success, on_error, .. } => IrStmt::Api {
                path: path.clone(),
                method: method.to_uppercase(),
                on_success: lower_stmts(on_success),
                on_error: lower_stmts(on_error),
            },
            Stmt::CallFn { name, args } => IrStmt::CallFn { name: name.clone(), args: args.clone() },
            Stmt::If { cond, then, els } => IrStmt::If {
                cond: cond.clone(),
                then: lower_stmts(then),
                els: lower_stmts(els),
            },
            Stmt::For { var, iter, body } => IrStmt::ForEach {
                var: var.clone(),
                iter: iter.clone(),
                body: lower_stmts(body),
            },
            Stmt::Add { value, target } => IrStmt::Add { target: target.clone(), value: value.clone() },
            Stmt::Remove { value, target } => IrStmt::Remove { target: target.clone(), value: value.clone() },
            Stmt::Return(expr) => IrStmt::Return(expr.clone()),
            _ => IrStmt::Nop,
        })
        .filter(|stmt| !matches!(stmt, _))
        .collect()
}

fn type_name(ty: &TypeRef) -> (String, bool) {
    match ty {
        TypeRef::Text => ("Text".into(), false),
        TypeRef::Number => ("Number".into(), false),
        TypeRef::Boolean => ("Boolean".into(), false),
        TypeRef::Date => ("Date".into(), false),
        TypeRef::Json => ("Json".into(), false),
        TypeRef::List(inner) => (format!("List<{}>", type_name(inner).0), false),
        TypeRef::Map(inner) => (format!("Map<{}>", type_name(inner).0), false),
        TypeRef::Optional(inner) => (type_name(inner).0, true),
        TypeRef::Named(name) => (name.clone(), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowers_accessibility_attributes() {
        let node = Node {
            kind: "button".into(),
            attrs: vec![
                ("label".into(), PropValue::Str("Wyślij".into())),
                ("accessibility".into(), PropValue::Str("Wyślij wiadomość".into())),
                ("height".into(), PropValue::Num(52.0)),
            ],
            children: vec![],
            on_tap: None,
        };
        let lowered = lower_node(&node);
        assert_eq!(lowered.label.as_deref(), Some("Wyślij"));
        assert_eq!(lowered.accessibility_label.as_deref(), Some("Wyślij wiadomość"));
        assert_eq!(lowered.height, Some(52.0));
    }

    #[test]
    fn resolves_navigation_target_without_params() {
        let stmts = lower_stmts(&[Stmt::Go { target: "Chat(id=1)".into() }]);
        assert_eq!(stmts, vec![IrStmt::Go { screen: "Chat".into() }]);
    }

    #[test]
    fn marks_optional_types() {
        assert_eq!(type_name(&TypeRef::Optional(Box::new(TypeRef::Text))), ("Text".into(), true));
        assert_eq!(type_name(&TypeRef::List(Box::new(TypeRef::Number))), ("List<Number>".into(), false));
    }
}
