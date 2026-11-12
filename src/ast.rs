use crate::types::Type;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Constant(String),
    Float(f64),
    Str(Arc<str>),
    Bool(bool),
    Variable(String),
    Call { name: String, args: Vec<Expr>, line: usize, column: usize },
    Unary { op: UnaryOp, operand: Box<Expr> },
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp { Negate, Not }

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add, Subtract, Multiply, Divide,
    Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub type_name: String,
    pub value: Expr,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>, // None = não devolve valor
    pub body: Vec<Stmt>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Show(Expr),
    Assign { name: String, declared_type: Option<Type>, value: Expr, line: usize, column: usize },
    Expr(Expr), // chamada usada como comando: Log($x);
    If { branches: Vec<(Expr, Vec<Stmt>)>, else_body: Option<Vec<Stmt>>, line: usize, column: usize },
    Return { value: Option<Expr>, line: usize, column: usize },
    Break,
    Continue,
    Stop,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub constants: Vec<ConstDecl>, // sempre lidas primeiro
    pub functions: Vec<FunctionDecl>,
    pub statements: Vec<Stmt>,
}