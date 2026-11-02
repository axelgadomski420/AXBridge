#[derive(Debug, Clone)]
pub struct Program {
    pub app: AppDecl,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct AppDecl {
    pub name: String,
    pub version: String,
    pub language: String,
    pub supports: Vec<String>,
    pub orientation: String,
    pub offline_first: bool,
}

#[derive(Debug, Clone)]
pub enum Item {
    Theme(Theme),
    Style(Style),
    Data(DataModel),
    Store(StoreDecl),
    GlobalState(Vec<GlobalVar>),
    Function(Function),
    Screen(Screen),
    Component(Component),
    When(WhenBlock),
    Timer(TimerBlock),
    Const(ConstDecl),
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub follows_system: bool,
    pub colors: Vec<(String, String)>, // (variant + name, hex)
    pub radii: Vec<(String, f64)>,
    pub fonts: Vec<(String, String)>,
    pub shadows: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Clone)]
pub struct Style {
    pub name: String,
    pub props: Vec<(String, PropValue)>,
}

#[derive(Debug, Clone)]
pub enum PropValue {
    Str(String),
    Num(f64),
    Color(String),
    Ident(String),
    List(Vec<String>),
    None,
}

#[derive(Debug, Clone)]
pub struct DataModel {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeRef,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TypeRef {
    Text,
    Number,
    Boolean,
    Date,
    Json,
    List(Box<TypeRef>),
    Map(Box<TypeRef>),
    Optional(Box<TypeRef>),
    Named(String),
}

#[derive(Debug, Clone)]
pub struct StoreDecl {
    pub name: String,
    pub scope: String, // "local" | "cloud"
    pub kind: StoreKind,
    pub ty: String,
    pub key: String,
    pub encrypted: bool,
}

#[derive(Debug, Clone)]
pub enum StoreKind { Single, List, Map }

#[derive(Debug, Clone)]
pub struct GlobalVar {
    pub name: String,
    pub ty: TypeRef,
    pub default: String,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<TypeRef>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Debug, Clone)]
pub struct Screen {
    pub name: String,
    pub params: Vec<Param>,
    pub title: Option<String>,
    pub states: Vec<StateVar>,
    pub body: Vec<Node>,
    pub handlers: Vec<Handler>,
}

#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct StateVar {
    pub name: String,
    pub ty: TypeRef,
    pub default: String,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub kind: String, // "column", "row", "text", "button", "image", ...
    pub attrs: Vec<(String, PropValue)>,
    pub children: Vec<Node>,
    pub on_tap: Option<Vec<Stmt>>,
}

#[derive(Debug, Clone)]
pub struct Handler {
    pub event: String, // "appears", "tap", "launch"
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct WhenBlock {
    pub event: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct TimerBlock {
    pub name: String,
    pub every: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub value: PropValue,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Set { target: String, value: Expr },
    Add { value: Expr, target: String },
    Remove { value: Expr, target: String },
    Clear { target: String },
    Load { target: String },
    Save { target: String },
    Update { target: String, field: String, value: Expr },
    Call {
        api: Option<String>,
        path: String,
        method: String,
        body: Option<Expr>,
        on_success: Vec<Stmt>,
        on_error: Vec<Stmt>,
    },
    CallFn { name: String, args: Vec<Expr> },
    Go { target: String },
    GoBack,
    ShowToast(Expr),
    ShowAlert(Expr),
    If { cond: Expr, then: Vec<Stmt>, els: Vec<Stmt> },
    For { var: String, iter: Expr, body: Vec<Stmt> },
    Return(Option<Expr>),
    Expr(Expr),
    Nop,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Str(String),
    Num(f64),
    Bool(bool),
    Ident(String),
    Field(Box<Expr>, String),
    Call(Box<Expr>, Vec<Expr>),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnOp(UnOp, Box<Expr>),
    Constructor { name: String, fields: Vec<(String, Expr)> },
    None,
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp { Add, Sub, Mul, Div, Eq, Ne, Lt, Le, Gt, Ge, And, Or }

#[derive(Debug, Clone, Copy)]
pub enum UnOp { Not, Neg }
