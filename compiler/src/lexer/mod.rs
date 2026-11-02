use crate::error::{AxError, AxResult};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Str(String),
    Num(f64),
    Color(String),      // #RRGGBB
    Indent,
    Dedent,
    Newline,
    Eof,

    LParen, RParen, LBracket, RBracket, LBrace, RBrace,
    Comma, Dot, Colon, Semi, Question,
    Assign, EqEq, NotEq,
    Plus, Minus, Star, Slash,
    Lt, LtEq, Gt, GtEq,
    Arrow,
    Pipe,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    indents: Vec<usize>,
    pending_dedents: usize,
    at_line_start: bool,
    out: Vec<Token>,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Self {
            chars: src.chars().collect(),
            pos: 0, line: 1, col: 1,
            indents: vec![0],
            pending_dedents: 0,
            at_line_start: true,
            out: Vec::new(),
        }
    }

    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn peek2(&self) -> Option<char> { self.chars.get(self.pos + 1).copied() }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' { self.line += 1; self.col = 1; }
        else { self.col += 1; }
        Some(c)
    }

    fn push(&mut self, tok: Tok) {
        self.out.push(Token { tok, line: self.line, col: self.col });
    }

    pub fn tokenize(mut self) -> AxResult<Vec<Token>> {
        while self.pos < self.chars.len() {
            if self.pending_dedents > 0 {
                self.push(Tok::Dedent);
                self.pending_dedents -= 1;
                continue;
            }
            if self.at_line_start {
                let emitted = self.handle_indent()?;
                if emitted { continue; }
                self.at_line_start = false;
            }
            let c = match self.peek() { Some(c) => c, None => break };

            // comments
            if c == '/' && self.peek2() == Some('/') {
                while let Some(ch) = self.peek() {
                    if ch == '\n' { break; }
                    self.bump();
                }
                continue;
            }
            // whitespace
            if c == ' ' || c == '\t' || c == '\r' { self.bump(); continue; }
            if c == '\n' {
                self.bump();
                self.push(Tok::Newline);
                self.at_line_start = true;
                continue;
            }
            // strings
            if c == '"' {
                self.bump();
                let mut s = String::new();
                while let Some(ch) = self.peek() {
                    if ch == '"' { self.bump(); break; }
                    if ch == '\\' {
                        self.bump();
                        if let Some(esc) = self.bump() {
                            s.push(match esc { 'n' => '\n', 't' => '\t', 'r' => '\r', '"' => '"', '\\' => '\\', o => o });
                        }
                    } else { s.push(ch); self.bump(); }
                }
                self.push(Tok::Str(s));
                continue;
            }
            // colors
            if c == '#' {
                self.bump();
                let mut hex = String::from("#");
                for _ in 0..8 {
                    if let Some(ch) = self.peek() {
                        if ch.is_ascii_hexdigit() { hex.push(ch); self.bump(); } else { break; }
                    }
                }
                self.push(Tok::Color(hex));
                continue;
            }
            // numbers
            if c.is_ascii_digit() {
                let mut n = String::new();
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() || ch == '.' || ch == '_' {
                        if ch != '_' { n.push(ch); }
                        self.bump();
                    } else { break; }
                }
                let v: f64 = n.parse().map_err(|_| AxError::lex(format!("zła liczba: {}", n), self.line, self.col))?;
                self.push(Tok::Num(v));
                continue;
            }
            // identifiers / keywords
            if c.is_alphabetic() || c == '_' {
                let mut id = String::new();
                while let Some(ch) = self.peek() {
                    if ch.is_alphanumeric() || ch == '_' { id.push(ch); self.bump(); } else { break; }
                }
                self.push(Tok::Ident(id));
                continue;
            }
            // punctuation
            self.bump();
            match c {
                '(' => self.push(Tok::LParen),
                ')' => self.push(Tok::RParen),
                '[' => self.push(Tok::LBracket),
                ']' => self.push(Tok::RBracket),
                '{' => self.push(Tok::LBrace),
                '}' => self.push(Tok::RBrace),
                ',' => self.push(Tok::Comma),
                '.' => self.push(Tok::Dot),
                ':' => self.push(Tok::Colon),
                ';' => self.push(Tok::Semi),
                '?' => self.push(Tok::Question),
                '+' => self.push(Tok::Plus),
                '*' => self.push(Tok::Star),
                '/' => self.push(Tok::Slash),
                '|' => self.push(Tok::Pipe),
                '=' => {
                    if self.peek() == Some('=') { self.bump(); self.push(Tok::EqEq); }
                    else { self.push(Tok::Assign); }
                }
                '!' => {
                    if self.peek() == Some('=') { self.bump(); self.push(Tok::NotEq); }
                    else { return Err(AxError::lex("'!' tylko jako '!='", self.line, self.col)); }
                }
                '<' => {
                    if self.peek() == Some('=') { self.bump(); self.push(Tok::LtEq); }
                    else { self.push(Tok::Lt); }
                }
                '>' => {
                    if self.peek() == Some('=') { self.bump(); self.push(Tok::GtEq); }
                    else { self.push(Tok::Gt); }
                }
                '-' => {
                    if self.peek() == Some('>') { self.bump(); self.push(Tok::Arrow); }
                    else { self.push(Tok::Minus); }
                }
                _ => return Err(AxError::lex(format!("nieoczekiwany znak '{}'", c), self.line, self.col)),
            }
        }
        while self.indents.len() > 1 {
            self.indents.pop();
            self.push(Tok::Dedent);
        }
        self.push(Tok::Eof);
        Ok(self.out)
    }

    fn handle_indent(&mut self) -> AxResult<bool> {
        let mut spaces = 0usize;
        while let Some(c) = self.peek() {
            match c {
                ' ' => { spaces += 1; self.bump(); }
                '\t' => { spaces += 4; self.bump(); }
                _ => break,
            }
        }
        // Empty line or comment-only
        match self.peek() {
            Some('\n') | None => { return Ok(false); }
            Some('/') if self.peek2() == Some('/') => { return Ok(false); }
            _ => {}
        }
        let current = *self.indents.last().unwrap();
        if spaces > current {
            self.indents.push(spaces);
            self.push(Tok::Indent);
            Ok(true)
        } else if spaces < current {
            while *self.indents.last().unwrap() > spaces {
                self.indents.pop();
                self.pending_dedents += 1;
            }
            if *self.indents.last().unwrap() != spaces {
                return Err(AxError::lex("niespójne wcięcie", self.line, self.col));
            }
            // Emit first dedent now
            if self.pending_dedents > 0 {
                self.push(Tok::Dedent);
                self.pending_dedents -= 1;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
