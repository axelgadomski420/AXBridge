//! Ultra Lexer for AXCode
//! Handles indentation, music/translate/provider keywords, comments, source maps

use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f]+")]
pub enum Token {
    // Keywords – core
    #[token("app")]
    App,
    #[token("version")]
    Version,
    #[token("language")]
    Language,
    #[token("supports")]
    Supports,
    #[token("screen")]
    Screen,
    #[token("component")]
    Component,
    #[token("state")]
    State,
    #[token("global")]
    Global,
    #[token("data")]
    Data,
    #[token("store")]
    Store,
    #[token("theme")]
    Theme,
    #[token("style")]
    Style,
    #[token("function")]
    Function,
    #[token("when")]
    When,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("list")]
    List,
    #[token("item")]
    Item,
    #[token("empty")]
    Empty,
    #[token("return")]
    Return,
    #[token("try")]
    Try,
    #[token("catch")]
    Catch,
    #[token("finally")]
    Finally,

    // UI primitives
    #[token("column")]
    Column,
    #[token("row")]
    Row,
    #[token("text")]
    Text,
    #[token("button")]
    Button,
    #[token("field")]
    Field,
    #[token("image")]
    Image,
    #[token("card")]
    Card,
    #[token("checkbox")]
    Checkbox,
    #[token("toggle")]
    Toggle,
    #[token("slider")]
    Slider,
    #[token("spinner")]
    Spinner,
    #[token("badge")]
    Badge,
    #[token("chip")]
    Chip,
    #[token("avatar")]
    Avatar,
    #[token("cameraPreview")]
    CameraPreview,
    #[token("floatingButton")]
    FloatingButton,
    #[token("sectionHeader")]
    SectionHeader,
    #[token("settingsItem")]
    SettingsItem,
    #[token("modeChip")]
    ModeChip,
    #[token("helpItem")]
    HelpItem,
    #[token("circle")]
    Circle,
    #[token("spacer")]
    Spacer,
    #[token("tab")]
    Tab,

    // Events & actions
    #[token("tapped")]
    Tapped,
    #[token("changed")]
    Changed,
    #[token("appears")]
    Appears,
    #[token("launches")]
    Launches,
    #[token("go")]
    Go,
    #[token("to")]
    To,
    #[token("back")]
    Back,
    #[token("set")]
    Set,
    #[token("add")]
    Add,
    #[token("remove")]
    Remove,
    #[token("update")]
    Update,
    #[token("save")]
    Save,
    #[token("load")]
    Load,
    #[token("show")]
    Show,
    #[token("alert")]
    Alert,
    #[token("toast")]
    Toast,
    #[token("notification")]
    Notification,

    // Ultra features
    #[token("translate")]
    Translate,
    #[token("music")]
    Music,
    #[token("generate")]
    Generate,
    #[token("play")]
    Play,
    #[token("reaction")]
    Reaction,
    #[token("persona")]
    Persona,
    #[token("provider")]
    Provider,
    #[token("tiktak")]
    TikTak,
    #[token("community")]
    Community,
    #[token("admin")]
    Admin,
    #[token("offline")]
    Offline,
    #[token("first")]
    First,
    #[token("encrypted")]
    Encrypted,
    #[token("sync")]
    Sync,
    #[token("online")]
    Online,

    // Types
    #[token("Text")]
    TypeText,
    #[token("Number")]
    TypeNumber,
    #[token("Boolean")]
    TypeBoolean,
    #[token("Date")]
    TypeDate,
    #[token("List")]
    TypeList,

    // Literals
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    StringLit(String),

    #[regex(r"[0-9]+\.?[0-9]*", |lex| lex.slice().to_string())]
    NumberLit(String),

    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("none")]
    None,
    #[token("null")]
    Null,

    // Identifiers
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // Symbols
    #[token("=")]
    Eq,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("?")]
    Question,

    // Comments & newlines / indentation handled separately
    #[regex(r"//[^\n]*", logos::skip)]
    Comment,

    #[token("\n")]
    Newline,

    #[error]
    Error,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub span: std::ops::Range<usize>,
    pub line: usize,
    pub col: usize,
}

pub fn tokenize(source: &str) -> Vec<TokenInfo> {
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut last_line_start = 0;

    let mut lexer = Token::lexer(source);

    while let Some(token) = lexer.next() {
        let span = lexer.span();
        // very simple line tracking
        let slice = &source[last_line_start..span.start];
        let newlines = slice.matches('\n').count();
        if newlines > 0 {
            line += newlines;
            last_line_start = span.start - slice.rfind('\n').map(|i| slice.len() - i - 1).unwrap_or(0);
        }
        let col = span.start - last_line_start + 1;

        if let Ok(t) = token {
            if t != Token::Newline {
                tokens.push(TokenInfo {
                    token: t,
                    span: span.start..span.end,
                    line,
                    col,
                });
            }
        }
    }
    tokens
}
