use crate::ast::*;
use crate::error::{AxError, AxResult};
use crate::lexer::{Lexer, Tok, Token};

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(src: &str) -> AxResult<Self> {
        let toks = Lexer::new(src).tokenize()?;
        Ok(Self { toks, pos: 0 })
    }

    fn peek(&self) -> &Tok { &self.toks[self.pos].tok }
    fn line(&self) -> usize { self.toks[self.pos].line }
    fn col(&self) -> usize { self.toks[self.pos].col }
    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if self.pos + 1 < self.toks.len() { self.pos += 1; }
        t
    }
    fn expect(&mut self, t: Tok) -> AxResult<()> {
        if self.peek() == &t { self.bump(); Ok(()) }
        else { Err(AxError::parse(format!("oczekiwano {:?}, mam {:?}", t, self.peek()), self.line(), self.col())) }
    }
    fn expect_ident(&mut self) -> AxResult<String> {
        match self.bump() {
            Tok::Ident(s) => Ok(s),
            other => Err(AxError::parse(format!("oczekiwano identyfikatora, mam {:?}", other), self.line(), self.col())),
        }
    }
    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Tok::Newline) { self.bump(); }
    }
    fn at_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Tok::Ident(s) if s == kw)
    }
    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.at_kw(kw) { self.bump(); true } else { false }
    }

    pub fn parse_program(&mut self) -> AxResult<Program> {
        self.skip_newlines();
        if !self.eat_kw("app") {
            return Err(AxError::parse("program musi zaczynać się od `app \"name\"`", self.line(), self.col()));
        }
        let name = match self.bump() {
            Tok::Str(s) => s,
            _ => return Err(AxError::parse("oczekiwano nazwy aplikacji w cudzysłowie", self.line(), self.col())),
        };
        self.expect(Tok::Newline)?;
        let mut app = AppDecl {
            name, version: "1.0.0".into(), language: "pl".into(),
            supports: vec![], orientation: "automatic".into(), offline_first: false,
        };
        // Parse app metadata at same indent
        loop {
            self.skip_newlines();
            if self.at_kw("version") { self.bump(); app.version = self.str_or_ident()?; }
            else if self.at_kw("language") { self.bump(); app.language = self.str_or_ident()?; }
            else if self.at_kw("supports") {
                self.bump();
                let mut v = vec![];
                while let Tok::Ident(_) = self.peek() { v.push(self.expect_ident()?); }
                app.supports = v;
            }
            else if self.at_kw("orientation") { self.bump(); app.orientation = self.str_or_ident()?; }
            else if self.at_kw("offline") { self.bump(); if self.eat_kw("first") { app.offline_first = true; } }
            else { break; }
            if matches!(self.peek(), Tok::Newline) { self.bump(); }
        }
        // Items
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Eof) { break; }
            let kw = match self.peek() { Tok::Ident(s) => s.clone(), _ => break };
            match kw.as_str() {
                "theme" => items.push(Item::Theme(self.parse_theme()?)),
                "style" => items.push(Item::Style(self.parse_style()?)),
                "data" => items.push(Item::Data(self.parse_data()?)),
                "store" => items.push(Item::Store(self.parse_store()?)),
                "global" => { self.bump(); let v = self.parse_global()?; items.push(Item::GlobalState(v)); }
                "function" => items.push(Item::Function(self.parse_function()?)),
                "screen" => items.push(Item::Screen(self.parse_screen()?)),
                "component" => items.push(Item::Component(self.parse_component()?)),
                "when" => items.push(Item::When(self.parse_when()?)),
                "timer" => items.push(Item::Timer(self.parse_timer()?)),
                "const" => items.push(Item::Const(self.parse_const()?)),
                _ => return Err(AxError::parse(format!("nieznana konstrukcja '{}'", kw), self.line(), self.col())),
            }
        }
        Ok(Program { app, items })
    }

    fn str_or_ident(&mut self) -> AxResult<String> {
        match self.bump() {
            Tok::Str(s) | Tok::Ident(s) => Ok(s),
            Tok::Num(n) => Ok(n.to_string()),
            other => Err(AxError::parse(format!("oczekiwano wartości, mam {:?}", other), self.line(), self.col())),
        }
    }

    fn parse_theme(&mut self) -> AxResult<Theme> {
        self.bump(); // theme
        let name = self.expect_ident()?;
        self.expect(Tok::Newline)?;
        self.expect(Tok::Indent)?;
        let mut th = Theme { name, follows_system: false, colors: vec![], radii: vec![], fonts: vec![], shadows: vec![] };
        loop {
            self.skip_newlines();
            if !matches!(self.peek(), Tok::Indent) && matches!(self.peek(), Tok::Dedent) { self.bump(); break; }
            match self.peek().clone() {
                Tok::Ident(kw) if kw == "follows" => {
                    self.bump(); self.expect_ident()?; th.follows_system = true;
                }
                Tok::Ident(kw) if kw == "color" => {
                    self.bump();
                    let mut variant = String::new();
                    if let Tok::Ident(_) = self.peek() {
                        // could be name or variant
                    }
                    let name = self.expect_ident()?;
                    let mut full_name = name.clone();
                    if let Tok::Ident(v) = self.peek().clone() {
                        if v != "light" && v != "dark" && !matches!(self.peek(), Tok::Newline) && !matches!(self.peek(), Tok::Color(_)) {
                            // likely variant
                        }
                    }
                    // Try variant
                    if matches!(self.peek(), Tok::Ident(_)) {
                        // check if next is color
                        let save = self.pos;
                        let variant = self.expect_ident()?;
                        if let Tok::Color(_) = self.peek() {
                            full_name = format!("{}_{}", name, variant);
                        } else {
                            self.pos = save;
                        }
                    }
                    if let Tok::Color(c) = self.bump() {
                        th.colors.push((full_name, c));
                    }
                    let _ = variant;
                }
                Tok::Ident(kw) if kw == "radius" => {
                    self.bump();
                    let n = self.expect_ident()?;
                    let v = match self.bump() { Tok::Num(n) => n, _ => 0.0 };
                    th.radii.push((n, v));
                }
                Tok::Ident(kw) if kw == "font" => {
                    self.bump();
                    let role = self.expect_ident()?;
                    let f = self.str_or_ident()?;
                    th.fonts.push((role, f));
                }
                Tok::Ident(kw) if kw == "shadow" => {
                    self.bump();
                    let n = self.expect_ident()?;
                    let mut parts = vec![];
                    loop {
                        match self.peek() {
                            Tok::Newline | Tok::Dedent => break,
                            Tok::Num(v) => { let v = *v; self.bump(); parts.push(v.to_string()); }
                            Tok::Color(c) => { let c = c.clone(); self.bump(); parts.push(c); }
                            _ => { self.bump(); }
                        }
                    }
                    th.shadows.push((n, parts));
                }
                _ => { self.bump(); }
            }
        }
        Ok(th)
    }

    fn parse_style(&mut self) -> AxResult<Style> {
        self.bump();
        let name = self.expect_ident()?;
        self.expect(Tok::Newline)?;
        self.expect(Tok::Indent)?;
        let mut props = vec![];
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Dedent) { self.bump(); break; }
            if matches!(self.peek(), Tok::Eof) { break; }
            let key = match self.peek() {
                Tok::Ident(s) => s.clone(),
                _ => { self.bump(); continue; }
            };
            self.bump();
            let val = self.parse_prop_value()?;
            props.push((key, val));
        }
        Ok(Style { name, props })
    }

    fn parse_prop_value(&mut self) -> AxResult<PropValue> {
        match self.peek().clone() {
            Tok::Str(s) => { self.bump(); Ok(PropValue::Str(s)) }
            Tok::Num(n) => { self.bump(); Ok(PropValue::Num(n)) }
            Tok::Color(c) => { self.bump(); Ok(PropValue::Color(c)) }
            Tok::Ident(s) => { self.bump(); Ok(PropValue::Ident(s)) }
            Tok::Newline | Tok::Dedent | Tok::Eof => Ok(PropValue::None),
            _ => { self.bump(); Ok(PropValue::None) }
        }
    }

    fn parse_data(&mut self) -> AxResult<DataModel> {
        self.bump();
        let name = self.expect_ident()?;
        self.expect(Tok::Newline)?;
        self.expect(Tok::Indent)?;
        let mut fields = vec![];
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Dedent) { self.bump(); break; }
            if matches!(self.peek(), Tok::Eof) { break; }
            let fname = self.expect_ident()?;
            let ty = self.parse_type()?;
            let mut default = None;
            if matches!(self.peek(), Tok::Assign) {
                self.bump();
                default = Some(self.str_or_ident()?);
            }
            fields.push(Field { name: fname, ty, default });
        }
        Ok(DataModel { name, fields })
    }

    fn parse_type(&mut self) -> AxResult<TypeRef> {
        let base = match self.bump() {
            Tok::Ident(s) => match s.as_str() {
                "Text" => TypeRef::Text,
                "Number" => TypeRef::Number,
                "Boolean" => TypeRef::Boolean,
                "Date" => TypeRef::Date,
                "Json" => TypeRef::Json,
                "List" => {
                    if matches!(self.peek(), Tok::Lt) {
                        self.bump();
                        let inner = self.parse_type()?;
                        self.expect(Tok::Gt)?;
                        TypeRef::List(Box::new(inner))
                    } else { TypeRef::Named(s) }
                }
                "Map" => {
                    if matches!(self.peek(), Tok::Lt) {
                        self.bump();
                        let inner = self.parse_type()?;
                        self.expect(Tok::Gt)?;
                        TypeRef::Map(Box::new(inner))
                    } else { TypeRef::Named(s) }
                }
                other => TypeRef::Named(other.to_string()),
            },
            other => return Err(AxError::parse(format!("oczekiwano typu, mam {:?}", other), self.line(), self.col())),
        };
        if matches!(self.peek(), Tok::Question) {
            self.bump();
            Ok(TypeRef::Optional(Box::new(base)))
        } else { Ok(base) }
    }

    fn parse_store(&mut self) -> AxResult<StoreDecl> {
        self.bump();
        let name = self.expect_ident()?;
        let scope = self.expect_ident()?;
        self.expect(Tok::Newline)?;
        self.expect(Tok::Indent)?;
        let mut kind = StoreKind::Single;
        let mut ty = String::new();
        let mut key = String::new();
        let mut encrypted = false;
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Dedent) { self.bump(); break; }
            if matches!(self.peek(), Tok::Eof) { break; }
            let kw = self.expect_ident()?;
            match kw.as_str() {
                "single" => { kind = StoreKind::Single; ty = self.expect_ident()?; }
                "list" => { kind = StoreKind::List; ty = self.expect_ident()?; }
                "map" => { kind = StoreKind::Map; ty = self.expect_ident()?; }
                "key" => { key = self.str_or_ident()?; }
                "encrypted" => { encrypted = self.str_or_ident()? == "true"; }
                _ => {}
            }
        }
        Ok(StoreDecl { name, scope, kind, ty, key, encrypted })
    }

    fn parse_global(&mut self) -> AxResult<Vec<GlobalVar>> {
        // `global state name Type = default` (already bumped `global`)
        let _ = self.eat_kw("state");
        let mut vars = vec![];
        loop {
            let name = self.expect_ident()?;
            let ty = self.parse_type()?;
            let mut default = String::new();
            if matches!(self.peek(), Tok::Assign) {
                self.bump();
                default = self.str_or_ident()?;
            }
            vars.push(GlobalVar { name, ty, default });
            if matches!(self.peek(), Tok::Newline) { self.bump(); } else { break; }
            // Next line must start with `global state`
            if !self.at_kw("global") { break; }
            self.bump();
            let _ = self.eat_kw("state");
        }
        Ok(vars)
    }

    fn parse_function(&mut self) -> AxResult<Function> {
        self.bump();
        let name = self.expect_ident()?;
        self.expect(Tok::LParen)?;
        let mut params = vec![];
        if !matches!(self.peek(), Tok::RParen) {
            loop {
                let pname = self.expect_ident()?;
                let pty = self.parse_type()?;
                params.push(Param { name: pname, ty: pty });
                if matches!(self.peek(), Tok::Comma) { self.bump(); } else { break; }
            }
        }
        self.expect(Tok::RParen)?;
        let mut ret = None;
        if self.eat_kw("returns") {
            ret = Some(self.parse_type()?);
        }
        self.expect(Tok::Newline)?;
        let body = self.parse_stmt_block()?;
        Ok(Function { name, params, ret, body })
    }

    fn parse_stmt_block(&mut self) -> AxResult<Vec<Stmt>> {
        if !matches!(self.peek(), Tok::Indent) { return Ok(vec![]); }
        self.bump();
        let mut stmts = vec![];
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Dedent) { self.bump(); break; }
            if matches!(self.peek(), Tok::Eof) { break; }
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> AxResult<Stmt> {
        let kw = match self.peek() {
            Tok::Ident(s) => s.clone(),
            _ => return self.parse_expr_stmt(),
        };
        match kw.as_str() {
            "set" => {
                self.bump();
                let target = self.parse_target()?;
                self.expect(Tok::Assign)?;
                let value = self.parse_expr()?;
                self.skip_newlines();
                Ok(Stmt::Set { target, value })
            }
            "add" => {
                self.bump();
                let value = self.parse_expr()?;
                self.skip_newlines();
                let mut target = String::new();
                if self.eat_kw("to") { target = self.expect_ident()?; }
                self.skip_newlines();
                Ok(Stmt::Add { value, target })
            }
            "remove" => {
                self.bump();
                let value = self.parse_expr()?;
                self.skip_newlines();
                let mut target = String::new();
                if self.eat_kw("from") { target = self.expect_ident()?; }
                self.skip_newlines();
                Ok(Stmt::Remove { value, target })
            }
            "clear" => { self.bump(); let t = self.expect_ident()?; self.skip_newlines(); Ok(Stmt::Clear { target: t }) }
            "load" => { self.bump(); let t = self.expect_ident()?; self.skip_newlines(); Ok(Stmt::Load { target: t }) }
            "save" => { self.bump(); let t = self.expect_ident()?; self.skip_newlines(); Ok(Stmt::Save { target: t }) }
            "update" => {
                self.bump();
                let t = self.expect_ident()?;
                self.expect(Tok::Dot)?;
                let f = self.expect_ident()?;
                self.expect(Tok::Assign)?;
                let v = self.parse_expr()?;
                self.skip_newlines();
                Ok(Stmt::Update { target: t, field: f, value: v })
            }
            "go" => {
                self.bump();
                if self.eat_kw("back") { self.skip_newlines(); return Ok(Stmt::GoBack); }
                self.eat_kw("to");
                let mut path = self.expect_ident()?;
                if matches!(self.peek(), Tok::LParen) {
                    let mut depth = 1; self.bump();
                    while depth > 0 {
                        match self.bump() {
                            Tok::LParen => depth += 1,
                            Tok::RParen => depth -= 1,
                            _ => {}
                        }
                    }
                }
                self.skip_newlines();
                Ok(Stmt::Go { target: path })
            }
            "call" => {
                self.bump();
                if self.eat_kw("api") {
                    let path = match self.bump() {
                        Tok::Str(s) => s,
                        Tok::Ident(s) => s,
                        _ => return Err(AxError::parse("oczekiwano ścieżki API", self.line(), self.col())),
                    };
                    let method = if self.eat_kw("method") { self.expect_ident()? } else { "GET".into() };
                    let body = if self.eat_kw("body") {
                        self.expect(Tok::LBrace)?;
                        // capture braces as raw expr — simplify: parse till RBrace
                        let mut _depth = 1;
                        let mut body_str = String::from("{");
                        while _depth > 0 {
                            match self.bump() {
                                Tok::LBrace => { _depth += 1; body_str.push('{'); }
                                Tok::RBrace => { _depth -= 1; if _depth > 0 { body_str.push('}'); } }
                                Tok::Newline => {}
                                Tok::Str(s) => { body_str.push('"'); body_str.push_str(&s); body_str.push('"'); }
                                Tok::Ident(s) => body_str.push_str(&s),
                                Tok::Colon => body_str.push(':'),
                                Tok::Comma => body_str.push(','),
                                Tok::Num(n) => body_str.push_str(&n.to_string()),
                                Tok::Dot => body_str.push('.'),
                                _ => {}
                            }
                        }
                        body_str.push('}');
                        Some(Expr::Ident(body_str))
                    } else { None };
                    self.skip_newlines();
                    let mut on_success = vec![];
                    let mut on_error = vec![];
                    // Check for `on success` / `on error` at NEXT deeper indent
                    while matches!(self.peek(), Tok::Indent | Tok::Dedent) || self.at_kw("on") {
                        if self.at_kw("on") {
                            self.bump();
                            let kind = self.expect_ident()?;
                            self.skip_newlines();
                            let body = self.parse_stmt_block()?;
                            if kind == "success" { on_success = body; }
                            else if kind == "error" { on_error = body; }
                        } else if matches!(self.peek(), Tok::Dedent) {
                            // peek next line for `on`
                            let save = self.pos;
                            self.bump();
                            self.skip_newlines();
                            if self.at_kw("on") { continue; }
                            self.pos = save;
                            break;
                        } else { break; }
                    }
                    Ok(Stmt::Call { api: Some("default".into()), path, method, body, on_success, on_error })
                } else {
                    let name = self.expect_ident()?;
                    self.expect(Tok::LParen)?;
                    let mut args = vec![];
                    if !matches!(self.peek(), Tok::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if matches!(self.peek(), Tok::Comma) { self.bump(); } else { break; }
                        }
                    }
                    self.expect(Tok::RParen)?;
                    self.skip_newlines();
                    Ok(Stmt::CallFn { name, args })
                }
            }
            "if" => {
                self.bump();
                let cond = self.parse_expr()?;
                self.skip_newlines();
                let then = self.parse_stmt_block()?;
                self.skip_newlines();
                let mut els = vec![];
                if self.eat_kw("else") {
                    self.skip_newlines();
                    els = self.parse_stmt_block()?;
                }
                Ok(Stmt::If { cond, then, els })
            }
            "for" => {
                self.bump();
                let var = self.expect_ident()?;
                self.eat_kw("in");
                let iter = self.parse_expr()?;
                self.skip_newlines();
                let body = self.parse_stmt_block()?;
                Ok(Stmt::For { var, iter, body })
            }
            "return" => {
                self.bump();
                if matches!(self.peek(), Tok::Newline) { self.skip_newlines(); return Ok(Stmt::Return(None)); }
                let e = self.parse_expr()?;
                self.skip_newlines();
                Ok(Stmt::Return(Some(e)))
            }
            "show" => {
                self.bump();
                if self.eat_kw("toast") {
                    let e = self.parse_expr()?;
                    self.skip_newlines();
                    return Ok(Stmt::ShowToast(e));
                }
                if self.eat_kw("alert") {
                    let e = self.parse_expr()?;
                    self.skip_newlines();
                    return Ok(Stmt::ShowAlert(e));
                }
                self.skip_newlines();
                Ok(Stmt::Nop)
            }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_target(&mut self) -> AxResult<String> {
        let mut path = self.expect_ident()?;
        while matches!(self.peek(), Tok::Dot) {
            self.bump();
            let f = self.expect_ident()?;
            path.push('.');
            path.push_str(&f);
        }
        Ok(path)
    }

    fn parse_expr_stmt(&mut self) -> AxResult<Stmt> {
        let e = self.parse_expr()?;
        self.skip_newlines();
        Ok(Stmt::Expr(e))
    }

    fn parse_expr(&mut self) -> AxResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> AxResult<Expr> {
        let mut l = self.parse_and()?;
        while self.at_kw("or") {
            self.bump();
            let r = self.parse_and()?;
            l = Expr::BinOp(Box::new(l), BinOp::Or, Box::new(r));
        }
        Ok(l)
    }
    fn parse_and(&mut self) -> AxResult<Expr> {
        let mut l = self.parse_eq()?;
        while self.at_kw("and") {
            self.bump();
            let r = self.parse_eq()?;
            l = Expr::BinOp(Box::new(l), BinOp::And, Box::new(r));
        }
        Ok(l)
    }
    fn parse_eq(&mut self) -> AxResult<Expr> {
        let mut l = self.parse_cmp()?;
        loop {
            match self.peek() {
                Tok::EqEq => { self.bump(); let r = self.parse_cmp()?; l = Expr::BinOp(Box::new(l), BinOp::Eq, Box::new(r)); }
                Tok::NotEq => { self.bump(); let r = self.parse_cmp()?; l = Expr::BinOp(Box::new(l), BinOp::Ne, Box::new(r)); }
                _ if self.at_kw("is") => {
                    self.bump();
                    let _ = self.eat_kw("not");
                    let r = self.parse_cmp()?;
                    l = Expr::BinOp(Box::new(l), BinOp::Eq, Box::new(r));
                }
                _ => break,
            }
        }
        Ok(l)
    }
    fn parse_cmp(&mut self) -> AxResult<Expr> {
        let mut l = self.parse_add()?;
        loop {
            match self.peek() {
                Tok::Lt => { self.bump(); let r = self.parse_add()?; l = Expr::BinOp(Box::new(l), BinOp::Lt, Box::new(r)); }
                Tok::LtEq => { self.bump(); let r = self.parse_add()?; l = Expr::BinOp(Box::new(l), BinOp::Le, Box::new(r)); }
                Tok::Gt => { self.bump(); let r = self.parse_add()?; l = Expr::BinOp(Box::new(l), BinOp::Gt, Box::new(r)); }
                Tok::GtEq => { self.bump(); let r = self.parse_add()?; l = Expr::BinOp(Box::new(l), BinOp::Ge, Box::new(r)); }
                _ => break,
            }
        }
        Ok(l)
    }
    fn parse_add(&mut self) -> AxResult<Expr> {
        let mut l = self.parse_mul()?;
        loop {
            match self.peek() {
                Tok::Plus => { self.bump(); let r = self.parse_mul()?; l = Expr::BinOp(Box::new(l), BinOp::Add, Box::new(r)); }
                Tok::Minus => { self.bump(); let r = self.parse_mul()?; l = Expr::BinOp(Box::new(l), BinOp::Sub, Box::new(r)); }
                _ => break,
            }
        }
        Ok(l)
    }
    fn parse_mul(&mut self) -> AxResult<Expr> {
        let mut l = self.parse_unary()?;
        loop {
            match self.peek() {
                Tok::Star => { self.bump(); let r = self.parse_unary()?; l = Expr::BinOp(Box::new(l), BinOp::Mul, Box::new(r)); }
                Tok::Slash => { self.bump(); let r = self.parse_unary()?; l = Expr::BinOp(Box::new(l), BinOp::Div, Box::new(r)); }
                _ => break,
            }
        }
        Ok(l)
    }
    fn parse_unary(&mut self) -> AxResult<Expr> {
        if self.at_kw("not") {
            self.bump();
            let e = self.parse_unary()?;
            return Ok(Expr::UnOp(UnOp::Not, Box::new(e)));
        }
        if matches!(self.peek(), Tok::Minus) {
            self.bump();
            let e = self.parse_unary()?;
            return Ok(Expr::UnOp(UnOp::Neg, Box::new(e)));
        }
        self.parse_postfix()
    }
    fn parse_postfix(&mut self) -> AxResult<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            if matches!(self.peek(), Tok::Dot) {
                self.bump();
                let f = self.expect_ident()?;
                e = Expr::Field(Box::new(e), f);
            } else if matches!(self.peek(), Tok::LParen) {
                self.bump();
                let mut args = vec![];
                if !matches!(self.peek(), Tok::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if matches!(self.peek(), Tok::Comma) { self.bump(); } else { break; }
                    }
                }
                self.expect(Tok::RParen)?;
                e = Expr::Call(Box::new(e), args);
            } else { break; }
        }
        Ok(e)
    }
    fn parse_primary(&mut self) -> AxResult<Expr> {
        match self.bump() {
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Ident(s) => match s.as_str() {
                "true" => Ok(Expr::Bool(true)),
                "false" => Ok(Expr::Bool(false)),
                "none" | "null" => Ok(Expr::None),
                _ => {
                    // Maybe constructor: Name(field=expr, ...)
                    if matches!(self.peek(), Tok::LParen) {
                        // heuristic: check if first token inside is Ident followed by '='
                        let save = self.pos;
                        self.bump();
                        let mut is_ctor = false;
                        if let Tok::Ident(_) = self.peek() {
                            let save2 = self.pos;
                            self.bump();
                            if matches!(self.peek(), Tok::Assign) { is_ctor = true; }
                            self.pos = save2;
                        }
                        if is_ctor {
                            let mut fields = vec![];
                            loop {
                                let fname = self.expect_ident()?;
                                self.expect(Tok::Assign)?;
                                let fval = self.parse_expr()?;
                                fields.push((fname, fval));
                                if matches!(self.peek(), Tok::Comma) { self.bump(); } else { break; }
                            }
                            self.expect(Tok::RParen)?;
                            return Ok(Expr::Constructor { name: s, fields });
                        }
                        self.pos = save;
                    }
                    Ok(Expr::Ident(s))
                }
            },
            Tok::LParen => {
                let e = self.parse_expr()?;
                self.expect(Tok::RParen)?;
                Ok(e)
            }
            Tok::LBrace => {
                // json object as ident
                let mut depth = 1;
                let mut buf = String::from("{");
                while depth > 0 {
                    match self.bump() {
                        Tok::LBrace => { depth += 1; buf.push('{'); }
                        Tok::RBrace => { depth -= 1; if depth > 0 { buf.push('}'); } }
                        Tok::Str(s) => { buf.push('"'); buf.push_str(&s); buf.push('"'); }
                        Tok::Ident(s) => buf.push_str(&s),
                        Tok::Colon => buf.push(':'),
                        Tok::Comma => buf.push(','),
                        Tok::Num(n) => buf.push_str(&n.to_string()),
                        Tok::Newline => {}
                        Tok::Eof => break,
                        _ => {}
                    }
                }
                buf.push('}');
                Ok(Expr::Ident(buf))
            }
            other => Err(AxError::parse(format!("nieoczekiwany token: {:?}", other), self.line(), self.col())),
        }
    }

    fn parse_screen(&mut self) -> AxResult<Screen> {
        self.bump();
        let name = self.expect_ident()?;
        let mut params = vec![];
        if matches!(self.peek(), Tok::LParen) {
            self.bump();
            if !matches!(self.peek(), Tok::RParen) {
                loop {
                    let pn = self.expect_ident()?;
                    let pt = self.parse_type()?;
                    params.push(Param { name: pn, ty: pt });
                    if matches!(self.peek(), Tok::Comma) { self.bump(); } else { break; }
                }
            }
            self.expect(Tok::RParen)?;
        }
        self.expect(Tok::Newline)?;
        self.expect(Tok::Indent)?;
        let mut screen = Screen { name, params, title: None, states: vec![], body: vec![], handlers: vec![] };
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Dedent) { self.bump(); break; }
            if matches!(self.peek(), Tok::Eof) { break; }
            if self.at_kw("title") {
                self.bump();
                if let Tok::Str(s) = self.bump() { screen.title = Some(s); }
            } else if self.at_kw("state") {
                self.bump();
                let n = self.expect_ident()?;
                let t = self.parse_type()?;
                let mut d = String::new();
                if matches!(self.peek(), Tok::Assign) { self.bump(); d = self.str_or_ident()?; }
                screen.states.push(StateVar { name: n, ty: t, default: d });
            } else if self.at_kw("when") {
                self.bump();
                let ev = self.str_or_ident()?;
                self.skip_newlines();
                let body = self.parse_stmt_block()?;
                screen.handlers.push(Handler { event: ev, body });
            } else {
                // UI node
                let node = self.parse_ui_node()?;
                screen.body.push(node);
            }
        }
        Ok(screen)
    }

    fn parse_ui_node(&mut self) -> AxResult<Node> {
        let kind = match self.peek() {
            Tok::Ident(s) => s.clone(),
            _ => return Err(AxError::parse("oczekiwano komponentu UI", self.line(), self.col())),
        };
        self.bump();
        let mut attrs = vec![];
        let mut label: Option<String> = None;
        // optional immediate string literal (button "X", text "Y")
        if let Tok::Str(s) = self.peek() {
            label = Some(s.clone());
            self.bump();
        } else if let Tok::Ident(s) = self.peek() {
            // button if X then "Y" else "Z" — capture whole line as label
            // Heuristic: if next token is not newline/indent, capture attrs
        }
        // attrs on same line
        loop {
            match self.peek() {
                Tok::Newline | Tok::Indent | Tok::Dedent | Tok::Eof => break,
                Tok::Ident(k) => {
                    let k = k.clone();
                    self.bump();
                    let v = self.parse_prop_value()?;
                    attrs.push((k, v));
                }
                _ => { self.bump(); }
            }
        }
        if let Some(l) = label {
            attrs.insert(0, ("label".into(), PropValue::Str(l)));
        }
        let mut children = vec![];
        let mut on_tap = None;
        if matches!(self.peek(), Tok::Indent) {
            self.bump();
            loop {
                self.skip_newlines();
                if matches!(self.peek(), Tok::Dedent) { self.bump(); break; }
                if matches!(self.peek(), Tok::Eof) { break; }
                if self.at_kw("when") {
                    self.bump();
                    let _ev = self.expect_ident()?;
                    // consume rest of line
                    while !matches!(self.peek(), Tok::Newline) { self.bump(); }
                    self.skip_newlines();
                    let body = self.parse_stmt_block()?;
                    on_tap = Some(body);
                } else if self.at_kw("alt") || self.at_kw("accessibility") {
                    self.bump();
                    // consume line
                    while !matches!(self.peek(), Tok::Newline) { self.bump(); }
                } else {
                    let c = self.parse_ui_node()?;
                    children.push(c);
                }
            }
        }
        Ok(Node { kind, attrs, children, on_tap })
    }

    fn parse_component(&mut self) -> AxResult<Component> {
        self.bump();
        let name = self.expect_ident()?;
        self.expect(Tok::LParen)?;
        let mut params = vec![];
        if !matches!(self.peek(), Tok::RParen) {
            loop {
                let pn = self.expect_ident()?;
                let pt = self.parse_type()?;
                params.push(Param { name: pn, ty: pt });
                if matches!(self.peek(), Tok::Comma) { self.bump(); } else { break; }
            }
        }
        self.expect(Tok::RParen)?;
        self.expect(Tok::Newline)?;
        self.expect(Tok::Indent)?;
        let mut body = vec![];
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Dedent) { self.bump(); break; }
            if matches!(self.peek(), Tok::Eof) { break; }
            body.push(self.parse_ui_node()?);
        }
        Ok(Component { name, params, body })
    }

    fn parse_when(&mut self) -> AxResult<WhenBlock> {
        self.bump();
        let mut event = String::new();
        // capture until newline
        while !matches!(self.peek(), Tok::Newline) {
            match self.bump() {
                Tok::Ident(s) => { if !event.is_empty() { event.push(' '); } event.push_str(&s); }
                Tok::Str(s) => event.push_str(&s),
                _ => {}
            }
        }
        self.skip_newlines();
        let body = self.parse_stmt_block()?;
        Ok(WhenBlock { event, body })
    }

    fn parse_timer(&mut self) -> AxResult<TimerBlock> {
        self.bump();
        let name = self.expect_ident()?;
        let mut every = String::new();
        if self.eat_kw("every") {
            every = self.str_or_ident()?;
        }
        self.skip_newlines();
        let body = self.parse_stmt_block()?;
        Ok(TimerBlock { name, every, body })
    }

    fn parse_const(&mut self) -> AxResult<ConstDecl> {
        self.bump();
        let name = self.expect_ident()?;
        self.expect(Tok::Assign)?;
        let value = self.parse_prop_value()?;
        self.skip_newlines();
        Ok(ConstDecl { name, value })
    }
}
