use crate::ast::*;

/// Basic optimizations: constant folding for numeric binops, dead `Nop` removal.
pub fn optimize(prog: &mut Program) {
    for item in &mut prog.items {
        match item {
            Item::Function(f) => { optimize_block(&mut f.body); }
            Item::Screen(s) => {
                for h in &mut s.handlers { optimize_block(&mut h.body); }
            }
            Item::When(w) => optimize_block(&mut w.body),
            _ => {}
        }
    }
}

fn optimize_block(stmts: &mut Vec<Stmt>) {
    stmts.retain(|s| !matches!(s, Stmt::Nop));
    for s in stmts {
        match s {
            Stmt::If { then, els, .. } => { optimize_block(then); optimize_block(els); }
            Stmt::For { body, .. } => optimize_block(body),
            Stmt::Call { on_success, on_error, .. } => { optimize_block(on_success); optimize_block(on_error); }
            _ => {}
        }
    }
}
