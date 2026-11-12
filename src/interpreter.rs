use crate::ast::*;
use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug)]
pub struct RuntimeError {
    pub message: String,
}

fn fail<T>(message: impl Into<String>) -> Result<T, RuntimeError> {
    Err(RuntimeError { message: message.into() })
}

pub struct Interpreter {
    constants: HashMap<String, Value>,
    variables: HashMap<String, Value>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self { constants: HashMap::new(), variables: HashMap::new() }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), RuntimeError> {
        // Constantes primeiro, como definido na linguagem
        for decl in &program.constants {
            let value = self.evaluate(&decl.value)?;
            self.constants.insert(decl.name.clone(), value);
        }
        for statement in &program.statements {
            self.execute(statement)?;
        }
        Ok(())
    }

    fn execute(&mut self, statement: &Stmt) -> Result<(), RuntimeError> {
        match statement {
            Stmt::Show(expr) => {
                let value = self.evaluate(expr)?;
                println!("{value}");
            }
            Stmt::Assign { name, value , .. } => {
                let value = self.evaluate(value)?;
                self.variables.insert(name.clone(), value);
            }
            // Ainda sem laços: entram quando criarmos o while e o Flow
            Stmt::Break | Stmt::Continue | Stmt::Stop => {
                return fail("break, continue and stop are not supported yet");
            }
        }
        Ok(())
    }

    fn evaluate(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(n) => Ok(Value::Float(*n)),
            Expr::Str(s) => Ok(Value::Str(s.as_ref().into())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Variable(name) => match self.variables.get(name) {
                Some(v) => Ok(v.clone()),
                None => fail(format!("undefined variable '${name}'")),
            },
            Expr::Constant(name) => match self.constants.get(name) {
                Some(v) => Ok(v.clone()),
                None => fail(format!("undefined constant '{name}'")),
            },
            Expr::Unary { op, operand } => {
                let value = self.evaluate(operand)?;
                match (op, value) {
                    (UnaryOp::Negate, Value::Int(n)) => Ok(Value::Int(-n)),
                    (UnaryOp::Negate, Value::Float(n)) => Ok(Value::Float(-n)),
                    (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (_, v) => fail(format!("cannot apply operator to {}", v.type_name())),
                }
            }
            Expr::Binary { left, op, right } => {
                let l = self.evaluate(left)?;
                let r = self.evaluate(right)?;
                binary(*op, l, r)
            }
        }
    }
}

fn binary(op: BinaryOp, l: Value, r: Value) -> Result<Value, RuntimeError> {
    use BinaryOp::*;
    use Value::*;

    match (op, l, r) {
        // Inteiros (tipagem forte: int com int)
        (Add, Int(a), Int(b)) => a.checked_add(b).map(Int).ok_or_else(overflow),
        (Subtract, Int(a), Int(b)) => a.checked_sub(b).map(Int).ok_or_else(overflow),
        (Multiply, Int(a), Int(b)) => a.checked_mul(b).map(Int).ok_or_else(overflow),
        (Divide, Int(_), Int(0)) => fail("division by zero"),
        (Divide, Int(a), Int(b)) => a.checked_div(b).map(Int).ok_or_else(overflow),

        // Floats
        (Add, Float(a), Float(b)) => Ok(Float(a + b)),
        (Subtract, Float(a), Float(b)) => Ok(Float(a - b)),
        (Multiply, Float(a), Float(b)) => Ok(Float(a * b)),
        (Divide, Float(a), Float(b)) => Ok(Float(a / b)),

        // Texto
        (Add, Str(a), Str(b)) => Ok(Str(format!("{a}{b}").into())),

        // Comparações
        (Equal, a, b) => Ok(Bool(a == b)),
        (NotEqual, a, b) => Ok(Bool(a != b)),
        (Less, Int(a), Int(b)) => Ok(Bool(a < b)),
        (LessEqual, Int(a), Int(b)) => Ok(Bool(a <= b)),
        (Greater, Int(a), Int(b)) => Ok(Bool(a > b)),
        (GreaterEqual, Int(a), Int(b)) => Ok(Bool(a >= b)),
        (Less, Float(a), Float(b)) => Ok(Bool(a < b)),
        (LessEqual, Float(a), Float(b)) => Ok(Bool(a <= b)),
        (Greater, Float(a), Float(b)) => Ok(Bool(a > b)),
        (GreaterEqual, Float(a), Float(b)) => Ok(Bool(a >= b)),

        (_, a, b) => fail(format!(
            "cannot apply operator to {} and {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

fn overflow() -> RuntimeError {
    RuntimeError { message: "integer overflow".to_string() }
}