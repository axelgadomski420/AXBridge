//! Recursive-descent parser with error recovery for Ultra AXCode

use crate::error::{AxError, AxResult};
use crate::lexer::{Token, TokenInfo};

#[derive(Debug, Clone)]
pub enum AstNode {
    App {
        name: String,
        version: String,
        language: String,
        supports: Vec<String>,
        body: Vec<AstNode>,
    },
    Screen {
        name: String,
        params: Vec<(String, String)>,
        body: Vec<AstNode>,
    },
    Component {
        name: String,
        params: Vec<(String, String)>,
        body: Vec<AstNode>,
    },
    Data {
        name: String,
        fields: Vec<(String, String)>,
    },
    Store {
        name: String,
        kind: String, // single | list
        data_type: String,
        key: String,
        encrypted: bool,
    },
    State {
        name: String,
        ty: String,
        value: Option<String>,
        is_global: bool,
    },
    Function {
        name: String,
        params: Vec<(String, String)>,
        body: Vec<AstNode>,
    },
    Theme {
        body: Vec<AstNode>,
    },
    Style {
        name: String,
        body: Vec<AstNode>,
    },
    // UI
    Column(Vec<AstNode>),
    Row(Vec<AstNode>),
    Text { value: String, style: Option<String> },
    Button { label: String, body: Vec<AstNode> },
    Field { name: String, props: Vec<(String, String)> },
    List { source: String, body: Vec<AstNode> },
    // Ultra
    Translate { expr: String, target: String },
    MusicGenerate { prompt: String, mood: String, provider: String },
    ProviderDecl { id: String, name: String, ty: String },
    PersonaSet { value: String },
    When { event: String, body: Vec<AstNode> },
    If { cond: String, then_body: Vec<AstNode>, else_body: Option<Vec<AstNode>> },
    Set { target: String, value: String },
    Call { name: String, args: Vec<String> },
    Raw(String), // fallback for yet-to-be-parsed constructs
}

pub struct Parser<'a> {
    tokens: &'a [TokenInfo],
    pos: usize,
    source: &'a str,
    file: String,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [TokenInfo], source: &'a str, file: &str) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
            file: file.to_string(),
        }
    }

    pub fn parse(&mut self) -> AxResult<AstNode> {
        // Very simplified top-level parse for the ultra compiler skeleton.
        // Real implementation would be a full recursive descent with indentation tracking.
        let mut body = Vec::new();

        while self.pos < self.tokens.len() {
            if let Some(node) = self.parse_toplevel()? {
                body.push(node);
            } else {
                self.pos += 1; // recovery
            }
        }

        Ok(AstNode::App {
            name: "AXBridge".into(),
            version: "1.2.0-ultra".into(),
            language: "pl".into(),
            supports: vec!["ios".into(), "android".into(), "web".into()],
            body,
        })
    }

    fn parse_toplevel(&mut self) -> AxResult<Option<AstNode>> {
        let tok = match self.current() {
            Some(t) => t,
            None => return Ok(None),
        };

        match &tok.token {
            Token::Screen => {
                self.bump();
                let name = self.expect_ident()?;
                Ok(Some(AstNode::Screen {
                    name,
                    params: vec![],
                    body: vec![],
                }))
            }
            Token::Component => {
                self.bump();
                let name = self.expect_ident()?;
                Ok(Some(AstNode::Component {
                    name,
                    params: vec![],
                    body: vec![],
                }))
            }
            Token::Data => {
                self.bump();
                let name = self.expect_ident()?;
                Ok(Some(AstNode::Data {
                    name,
                    fields: vec![],
                }))
            }
            Token::Function => {
                self.bump();
                let name = self.expect_ident()?;
                Ok(Some(AstNode::Function {
                    name,
                    params: vec![],
                    body: vec![],
                }))
            }
            _ => Ok(None),
        }
    }

    fn current(&self) -> Option<&TokenInfo> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn expect_ident(&mut self) -> AxResult<String> {
        if let Some(TokenInfo { token: Token::Ident(name), .. }) = self.current() {
            let n = name.clone();
            self.bump();
            Ok(n)
        } else {
            Err(AxError::Parse {
                message_pl: "Oczekiwano identyfikatora".into(),
                message_en: "Expected identifier".into(),
                span: self.current().map(|t| t.span.clone()).unwrap_or(0..0),
                file: self.file.clone(),
                suggestion: None,
            })
        }
    }
}

pub fn parse_file(source: &str, file: &str) -> AxResult<AstNode> {
    let tokens = crate::lexer::tokenize(source);
    let mut parser = Parser::new(&tokens, source, file);
    parser.parse()
}
