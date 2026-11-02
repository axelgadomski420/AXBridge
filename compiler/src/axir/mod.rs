//! AX Intermediate Representation — thin layer between AST and codegen.
use crate::ast::*;

#[derive(Debug)]
pub struct Ir {
    pub app: AppDecl,
    pub models: Vec<DataModel>,
    pub stores: Vec<StoreDecl>,
    pub globals: Vec<GlobalVar>,
    pub functions: Vec<Function>,
    pub screens: Vec<Screen>,
    pub components: Vec<Component>,
    pub handlers: Vec<WhenBlock>,
}

pub fn lower(prog: &Program) -> Ir {
    let mut ir = Ir {
        app: prog.app.clone(),
        models: vec![], stores: vec![], globals: vec![],
        functions: vec![], screens: vec![], components: vec![], handlers: vec![],
    };
    for item in &prog.items {
        match item {
            Item::Data(d) => ir.models.push(d.clone()),
            Item::Store(s) => ir.stores.push(s.clone()),
            Item::GlobalState(g) => ir.globals.extend(g.clone()),
            Item::Function(f) => ir.functions.push(f.clone()),
            Item::Screen(s) => ir.screens.push(s.clone()),
            Item::Component(c) => ir.components.push(c.clone()),
            Item::When(w) => ir.handlers.push(w.clone()),
            _ => {}
        }
    }
    ir
}
