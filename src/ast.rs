use std::sync::Arc;
use crate::types::Type;
//use std::mem::type_info::Type;
#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Constant(String),
    Float(f64),
    Str(Arc<str>),
    Bool(bool),
    Variable(String),
    Unary { op: UnaryOp, operand: Box<Expr> },
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
}

#[derive(Debug, Clone)]
pub struct Program {
    pub constants: Vec<ConstDecl>, // sempre lidas primeiro
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp { Negate, Not }

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add, Subtract, Multiply, Divide,
    Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual,
}

// novos tipos:
#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub type_name: String,
    pub value: Expr,
    pub line: usize,
    pub column: usize,
}




#[derive(Debug, Clone)]
pub enum Stmt {
    Show(Expr),
    Assign { name: String, declared_type: Option<Type>, value: Expr, line: usize, column: usize },
    Break,
    Continue,
    Stop,
}