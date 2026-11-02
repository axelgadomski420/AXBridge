use std::fmt;

pub type AxResult<T> = Result<T, AxError>;

#[derive(Debug, Clone, Copy)]
pub enum AxErrorKind { Lex, Parse, Semantic, Codegen, Io, Internal }

#[derive(Debug)]
pub struct AxError {
    pub kind: AxErrorKind,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl AxError {
    pub fn new(k: AxErrorKind, msg: impl Into<String>, line: usize, col: usize) -> Self {
        Self { kind: k, message: msg.into(), line, column: col }
    }
    pub fn lex(msg: impl Into<String>, l: usize, c: usize) -> Self { Self::new(AxErrorKind::Lex, msg, l, c) }
    pub fn parse(msg: impl Into<String>, l: usize, c: usize) -> Self { Self::new(AxErrorKind::Parse, msg, l, c) }
    pub fn semantic(msg: impl Into<String>, l: usize, c: usize) -> Self { Self::new(AxErrorKind::Semantic, msg, l, c) }
    pub fn codegen(msg: impl Into<String>) -> Self { Self::new(AxErrorKind::Codegen, msg, 0, 0) }
    pub fn io(msg: impl Into<String>) -> Self { Self::new(AxErrorKind::Io, msg, 0, 0) }
}

impl fmt::Display for AxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "[{:?}] {} ({}:{})", self.kind, self.message, self.line, self.column)
        } else {
            write!(f, "[{:?}] {}", self.kind, self.message)
        }
    }
}

impl std::error::Error for AxError {}

impl From<std::io::Error> for AxError {
    fn from(e: std::io::Error) -> Self { AxError::io(e.to_string()) }
}
