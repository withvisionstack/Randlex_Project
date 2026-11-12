use crate::ast::*;
use crate::value::Value;
use std::collections::HashMap;
use std::rc::Rc;

const MAX_CALL_DEPTH: usize = 200;

#[derive(Debug)]
pub struct RuntimeError {
    pub message: String,
    pub stop: bool, // true = não é erro, é um `stop` subindo pela pilha
}

impl RuntimeError {
    fn stop() -> Self {
        Self { message: String::new(), stop: true }
    }
}

fn fail<T>(message: impl Into<String>) -> Result<T, RuntimeError> {
    Err(RuntimeError { message: message.into(), stop: false })
}

enum Flow {
    Normal,
    Break,
    Continue,
    Return(Option<Value>),
    Stop,
}

pub struct Interpreter {
    constants: HashMap<String, Value>,
    variables: HashMap<String, Value>,
    functions: HashMap<String, Rc<FunctionDecl>>,
    depth: usize,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            variables: HashMap::new(),
            functions: HashMap::new(),
            depth: 0,
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), RuntimeError> {
        match self.run_program(program) {
            Err(e) if e.stop => Ok(()), // `stop` encerrou o programa normalmente
            other => other,
        }
    }

    fn run_program(&mut self, program: &Program) -> Result<(), RuntimeError> {
        for decl in &program.constants {
            let value = self.evaluate(&decl.value)?;
            self.constants.insert(decl.name.clone(), value);
        }
        for function in &program.functions {
            self.functions.insert(function.name.clone(), Rc::new(function.clone()));
        }
        for statement in &program.statements {
            if let Flow::Stop = self.execute(statement)? {
                return Ok(());
            }
        }
        Ok(())
    }

    fn execute_block(&mut self, body: &[Stmt]) -> Result<Flow, RuntimeError> {
        for statement in body {
            let flow = self.execute(statement)?;
            if !matches!(flow, Flow::Normal) {
                return Ok(flow); // Return/Break/Continue/Stop sobem até quem trata
            }
        }
        Ok(Flow::Normal)
    }

    fn execute(&mut self, statement: &Stmt) -> Result<Flow, RuntimeError> {
        match statement {
            Stmt::Show(expr) => {
                let value = self.evaluate(expr)?;
                println!("{value}");
                Ok(Flow::Normal)
            }
            Stmt::Assign { name, value, .. } => {
                let value = self.evaluate(value)?;
                self.variables.insert(name.clone(), value);
                Ok(Flow::Normal)
            }
            Stmt::Expr(expr) => {
                self.evaluate(expr)?;
                Ok(Flow::Normal)
            }
            Stmt::If { branches, else_body, .. } => {
                for (condition, body) in branches {
                    match self.evaluate(condition)? {
                        Value::Bool(true) => return self.execute_block(body),
                        Value::Bool(false) => {}
                        other => return fail(format!("condition must be bool, found {}", other.type_name())),
                    }
                }
                match else_body {
                    Some(body) => self.execute_block(body),
                    None => Ok(Flow::Normal),
                }
            }
            Stmt::Return { value, .. } => {
                let value = match value {
                    Some(expr) => Some(self.evaluate(expr)?),
                    None => None,
                };
                Ok(Flow::Return(value))
            }
            Stmt::Break => Ok(Flow::Break),
            Stmt::Continue => Ok(Flow::Continue),
            Stmt::Stop => Ok(Flow::Stop),
        }
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> Result<Value, RuntimeError> {
        let Some(function) = self.functions.get(name).cloned() else {
            return fail(format!("undefined function '{name}'"));
        };

        // argumentos são avaliados no escopo de quem chama
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.evaluate(arg)?);
        }

        if self.depth >= MAX_CALL_DEPTH {
            return fail("stack overflow: too many nested calls");
        }

        let mut locals = HashMap::new();
        for (param, value) in function.params.iter().zip(values) {
            locals.insert(param.name.clone(), value);
        }

        let caller_variables = std::mem::replace(&mut self.variables, locals);
        self.depth += 1;
        let flow = self.execute_block(&function.body);
        self.depth -= 1;
        self.variables = caller_variables;

        match flow? {
            Flow::Return(Some(value)) => Ok(value),
            Flow::Stop => Err(RuntimeError::stop()),
            _ => Ok(Value::Unit),
        }
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
            Expr::Call { name, args, .. } => self.call(name, args),
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

// === daqui para baixo: igual ao que você já tem ===
// Mantenha as funções `binary` e `overflow` exatamente como estavam,
// mas troque o construtor em `overflow` para:
//     RuntimeError { message: "integer overflow".to_string(), stop: false }


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
    RuntimeError { message: "integer overflow".to_string(), stop: false }
}