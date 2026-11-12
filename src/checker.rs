use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::types::Type;
use std::collections::HashMap;

#[derive(Clone)]
struct FunctionSig {
    params: Vec<Type>,
    ret: Option<Type>,
}

pub struct Checker {
    constants: HashMap<String, Type>,
    variables: HashMap<String, Type>,
    functions: HashMap<String, FunctionSig>,
    // None = fora de função; Some(tipo_de_retorno) = dentro de uma função
    current_return: Option<Option<Type>>,
    line: usize,
    column: usize,
}

impl Checker {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            variables: HashMap::new(),
            functions: HashMap::new(),
            current_return: None,
            line: 1,
            column: 1,
        }
    }

    pub fn check(mut self, program: &Program) -> Result<(), Diagnostic> {
        for decl in &program.constants {
            self.line = decl.line;
            self.column = decl.column;
            let Some(ty) = Type::from_name(&decl.type_name) else {
                return Err(self.err("RX300", format!("unknown type '{}'", decl.type_name)));
            };
            let found = self.infer(&decl.value, Some(ty))?;
            self.expect_type(ty, found)?;
            self.constants.insert(decl.name.clone(), ty);
        }

        // 1) registra todas as assinaturas (permite chamar antes de declarar e recursão)
        for f in &program.functions {
            self.line = f.line;
            self.column = f.column;
            let sig = FunctionSig {
                params: f.params.iter().map(|p| p.ty).collect(),
                ret: f.return_type,
            };
            if self.functions.insert(f.name.clone(), sig).is_some() {
                return Err(self.err("RX307", format!("function '{}' is already declared", f.name)));
            }
        }

        // 2) confere o corpo de cada função
        for f in &program.functions {
            self.check_function(f)?;
        }

        // 3) confere o código do nível de cima
        for statement in &program.statements {
            self.check_statement(statement)?;
        }
        Ok(())
    }

    fn check_function(&mut self, f: &FunctionDecl) -> Result<(), Diagnostic> {
        self.line = f.line;
        self.column = f.column;

        let outer = std::mem::take(&mut self.variables); // escopo próprio
        for p in &f.params {
            if self.variables.insert(p.name.clone(), p.ty).is_some() {
                return Err(self.err("RX308", format!("duplicate parameter '${}'", p.name)));
            }
        }
        self.current_return = Some(f.return_type);

        self.check_block(&f.body)?;

        if f.return_type.is_some() && !body_always_returns(&f.body) {
            self.line = f.line;
            self.column = f.column;
            return Err(self
                .err("RX312", format!("function '{}' does not return a value on every path", f.name))
                .help("add a 'return' at the end, or an 'else' branch that returns"));
        }

        self.current_return = None;
        self.variables = outer;
        Ok(())
    }

    // Variáveis declaradas dentro do bloco deixam de existir no fim dele
    fn check_block(&mut self, body: &[Stmt]) -> Result<(), Diagnostic> {
        let saved = self.variables.clone();
        for statement in body {
            self.check_statement(statement)?;
        }
        self.variables = saved;
        Ok(())
    }

    fn check_statement(&mut self, statement: &Stmt) -> Result<(), Diagnostic> {
        match statement {
            Stmt::Show(expr) => {
                self.infer(expr, None)?;
            }
            Stmt::Expr(expr) => match expr {
                Expr::Call { name, args, line, column } => {
                    self.check_call(name, args, *line, *column)?;
                }
                other => {
                    self.infer(other, None)?;
                }
            },
            Stmt::Assign { name, declared_type, value, line, column } => {
                self.line = *line;
                self.column = *column;
                match (self.variables.get(name).copied(), *declared_type) {
                    (Some(_), Some(_)) => {
                        return Err(self
                            .err("RX301", format!("variable '${name}' is already declared"))
                            .help("remove the type to assign a new value, example: $x = 1;"));
                    }
                    (Some(existing), None) => {
                        let found = self.infer(value, Some(existing))?;
                        self.line = *line;
                        self.column = *column;
                        self.expect_type(existing, found)?;
                    }
                    (None, Some(ty)) => {
                        let found = self.infer(value, Some(ty))?;
                        self.line = *line;
                        self.column = *column;
                        self.expect_type(ty, found)?;
                        self.variables.insert(name.clone(), ty);
                    }
                    (None, None) => {
                        return Err(self
                            .err("RX306", format!("variable '${name}' used before being declared"))
                            .help(format!("declare it with a type: ${name}: int64 = ...;"))
                            .help("only '$name: type = value;' declares a variable; '$name = value;' only changes an existing one"));
                    }
                }
            }
            Stmt::If { branches, else_body, line, column } => {
                for (condition, body) in branches {
                    let ty = self.infer(condition, Some(Type::Bool))?;
                    self.line = *line;
                    self.column = *column;
                    self.expect_type(Type::Bool, ty)?;
                    self.check_block(body)?;
                }
                if let Some(body) = else_body {
                    self.check_block(body)?;
                }
            }
            Stmt::Return { value, line, column } => {
                self.line = *line;
                self.column = *column;
                let Some(expected) = self.current_return else {
                    return Err(self.err("RX311", "'return' is only allowed inside a function"));
                };
                match (expected, value) {
                    (Some(ty), Some(expr)) => {
                        let found = self.infer(expr, Some(ty))?;
                        self.line = *line;
                        self.column = *column;
                        self.expect_type(ty, found)?;
                    }
                    (Some(ty), None) => {
                        return Err(self
                            .err("RX313", format!("this function must return a value of type {}", ty.name())));
                    }
                    (None, Some(_)) => {
                        return Err(self
                            .err("RX313", "this function has no return type, so it cannot return a value")
                            .help("add a return type: function Name(...): int64 { ... }"));
                    }
                    (None, None) => {}
                }
            }
            Stmt::Break | Stmt::Continue | Stmt::Stop => {}
        }
        Ok(())
    }

    fn check_call(&mut self, name: &str, args: &[Expr], line: usize, column: usize) -> Result<Option<Type>, Diagnostic> {
        self.line = line;
        self.column = column;
        let Some(sig) = self.functions.get(name).cloned() else {
            return Err(self.err("RX309", format!("undefined function '{name}'")));
        };
        if args.len() != sig.params.len() {
            return Err(self.err(
                "RX310",
                format!("function '{name}' takes {} argument(s), but {} were given", sig.params.len(), args.len()),
            ));
        }
        for (arg, expected) in args.iter().zip(&sig.params) {
            let found = self.infer(arg, Some(*expected))?;
            self.line = line;
            self.column = column;
            self.expect_type(*expected, found)?;
        }
        Ok(sig.ret)
    }

    fn err(&self, code: &'static str, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(code, message, self.line, self.column)
    }

    fn expect_type(&self, expected: Type, found: Type) -> Result<(), Diagnostic> {
        if expected == found {
            return Ok(());
        }
        Err(self
            .err("RX302", format!("mismatched types: expected {}, found {}", expected.name(), found.name()))
            .help("Rendlex never converts types silently"))
    }

    fn int_literal(&self, value: i128, expected: Option<Type>) -> Result<Type, Diagnostic> {
        match expected {
            Some(ty) if ty.is_integer() => {
                let (min, max) = ty.int_range().unwrap();
                if value < min || value > max {
                    return Err(self
                        .err("RX303", format!("literal {value} is out of range for {}", ty.name()))
                        .help(format!("{} accepts values from {min} to {max}", ty.name())));
                }
                Ok(ty)
            }
            _ => Ok(Type::Int64),
        }
    }

    fn infer(&mut self, expr: &Expr, expected: Option<Type>) -> Result<Type, Diagnostic> {
        match expr {
            Expr::Int(n) => self.int_literal(*n as i128, expected),
            Expr::Float(_) => Ok(if expected == Some(Type::Float32) { Type::Float32 } else { Type::Float64 }),
            Expr::Str(_) => Ok(Type::Str),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Variable(name) => self
                .variables
                .get(name)
                .copied()
                .ok_or_else(|| self.err("RX304", format!("undefined variable '${name}'"))),
            Expr::Constant(name) => self
                .constants
                .get(name)
                .copied()
                .ok_or_else(|| self.err("RX304", format!("undefined constant '{name}'"))),
            Expr::Call { name, args, line, column } => match self.check_call(name, args, *line, *column)? {
                Some(ty) => Ok(ty),
                None => Err(self.err("RX314", format!("function '{name}' does not return a value"))),
            },
            Expr::Unary { op, operand } => match op {
                UnaryOp::Negate => {
                    if let Expr::Int(n) = &**operand {
                        return self.int_literal(-(*n as i128), expected);
                    }
                    let ty = self.infer(operand, expected)?;
                    if !ty.is_numeric() {
                        return Err(self.err("RX305", format!("cannot negate {}", ty.name())));
                    }
                    Ok(ty)
                }
                UnaryOp::Not => {
                    let ty = self.infer(operand, Some(Type::Bool))?;
                    self.expect_type(Type::Bool, ty)?;
                    Ok(Type::Bool)
                }
            },
            Expr::Binary { left, op, right } => {
                use BinaryOp::*;
                let is_comparison = matches!(op, Equal | NotEqual | Less | LessEqual | Greater | GreaterEqual);
                let (lt, rt) = self.infer_pair(left, right, if is_comparison { None } else { expected })?;
                if lt != rt {
                    return Err(self
                        .err("RX302", format!("mismatched types: {} and {}", lt.name(), rt.name()))
                        .help("Rendlex never converts types silently"));
                }
                match op {
                    Add if lt == Type::Str => Ok(Type::Str),
                    Add | Subtract | Multiply | Divide if lt.is_numeric() => Ok(lt),
                    Equal | NotEqual => Ok(Type::Bool),
                    Less | LessEqual | Greater | GreaterEqual if lt.is_numeric() => Ok(Type::Bool),
                    _ => Err(self.err("RX305", format!("operator cannot be applied to {}", lt.name()))),
                }
            }
        }
    }

    fn infer_pair(&mut self, left: &Expr, right: &Expr, expected: Option<Type>) -> Result<(Type, Type), Diagnostic> {
        if is_literal(left) && !is_literal(right) {
            let rt = self.infer(right, expected)?;
            let lt = self.infer(left, Some(rt))?;
            Ok((lt, rt))
        } else {
            let lt = self.infer(left, expected)?;
            let rt = self.infer(right, Some(lt))?;
            Ok((lt, rt))
        }
    }
}

fn is_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Int(_) | Expr::Float(_))
}

// O corpo garante retorno se algum comando dele sempre retorna
fn body_always_returns(body: &[Stmt]) -> bool {
    body.iter().any(stmt_always_returns)
}

fn stmt_always_returns(statement: &Stmt) -> bool {
    match statement {
        Stmt::Return { .. } | Stmt::Stop => true,
        // `if` só garante retorno se tiver `else` e TODOS os ramos retornarem
        Stmt::If { branches, else_body: Some(else_body), .. } => {
            branches.iter().all(|(_, body)| body_always_returns(body)) && body_always_returns(else_body)
        }
        _ => false,
    }
}