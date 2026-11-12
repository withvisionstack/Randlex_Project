use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::types::Type;
use std::collections::HashMap;

pub struct Checker {
    constants: HashMap<String, Type>,
    variables: HashMap<String, Type>,
    line: usize,
    column: usize,
}

impl Checker {
    pub fn new() -> Self {
        Self { constants: HashMap::new(), variables: HashMap::new(), line: 1, column: 1 }
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
        for statement in &program.statements {
            self.check_statement(statement)?;
        }
        Ok(())
    }

    fn check_statement(&mut self, statement: &Stmt) -> Result<(), Diagnostic> {
        match statement {
            Stmt::Show(expr) => {
                self.infer(expr, None)?;
            }
            Stmt::Assign { name, declared_type, value, line, column } => {
                self.line = *line;
                self.column = *column;
                match (self.variables.get(name).copied(), *declared_type) {
                    (Some(_), Some(_)) => {
                        return Err(self.err("RX301", format!("variable '${name}' is already declared"))
                            .help("remove the type to assign a new value, example: $x = 1;"));
                    }
                    (Some(existing), None) => {
                        let found = self.infer(value, Some(existing))?;
                        self.expect_type(existing, found)?;
                    }
                    (None, Some(ty)) => {
                        let found = self.infer(value, Some(ty))?;
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
            Stmt::Break | Stmt::Continue | Stmt::Stop => {}
        }
        Ok(())
    }

    fn err(&self, code: &'static str, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(code, message, self.line, self.column)
    }

    fn expect_type(&self, expected: Type, found: Type) -> Result<(), Diagnostic> {
        if expected == found {
            return Ok(());
        }
        Err(self.err(
            "RX302",
            format!("mismatched types: expected {}, found {}", expected.name(), found.name()),
        )
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
            _ => Ok(Type::Int64), // sem tipo esperado: padrão
        }
    }

    // Descobre o tipo de uma expressão. `expected` ajuda os literais a assumirem o tipo certo.
    fn infer(&mut self, expr: &Expr, expected: Option<Type>) -> Result<Type, Diagnostic> {
        match expr {
            Expr::Int(n) => self.int_literal(*n as i128, expected),
            Expr::Float(_) => Ok(if expected == Some(Type::Float32) { Type::Float32 } else { Type::Float64 }),
            Expr::Str(_) => Ok(Type::Str),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Variable(name) => self.variables.get(name).copied().ok_or_else(|| {
                self.err("RX304", format!("undefined variable '${name}'"))
            }),
            Expr::Constant(name) => self.constants.get(name).copied().ok_or_else(|| {
                self.err("RX304", format!("undefined constant '{name}'"))
            }),
            Expr::Unary { op, operand } => match op {
                UnaryOp::Negate => {
                    // -128 é válido em int8, mesmo que 128 sozinho não seja
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
                    return Err(self.err(
                        "RX302",
                        format!("mismatched types: {} and {}", lt.name(), rt.name()),
                    )
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

    // Um literal se adapta ao tipo do outro lado: em `$x + 1` o 1 vira o tipo de $x
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