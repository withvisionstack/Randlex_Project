use crate::ast::*;
use crate::token::{Token, TokenKind};
use crate::diagnostic::Diagnostic;
use crate::types::Type;
//use std::mem::type_info::Type;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    loop_depth: usize,
}



impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, position: 0, loop_depth: 0 }
    }

    pub fn parse_program(mut self) -> Result<Program, Diagnostic> {
        let mut constants = Vec::new();
        while self.current().kind == TokenKind::Const {
            constants.push(self.parse_const()?);
        }



        let mut statements = Vec::new();
        while !self.is_at_end() {
            if self.current().kind == TokenKind::Const {
                return Err(self.error_code("RX101", "constants must be declared at the top of the file"));
            }
            statements.push(self.parse_statement()?);
        }
        Ok(Program { constants, statements })
    }

    fn parse_const(&mut self) -> Result<ConstDecl, Diagnostic>  {
        let start = self.advance(); // consome `const`


        let name = match self.current().kind.clone() {
            TokenKind::Identifier(n) => n,
            _ => return Err(self.error("expected a constant name")),
        };
        if !is_uppercase_name(&name) {
            return Err(self.error_code("RX102", "constant names must be UPPERCASE (example: MAX_SIZE)"));
        }
        self.advance();

        self.expect(TokenKind::Colon, "expected ':' and a type after the constant name")?;
        let type_name = match self.current().kind.clone() {
            TokenKind::Identifier(t) => t,
            _ => return Err(self.error("expected a type name")),
        };
        self.advance();

        self.expect(TokenKind::Assign, "expected '=' before the constant value")?;
        let value = self.parse_expression()?;
        self.expect(TokenKind::Semicolon, "expected ';' after constant")?;

        Ok(ConstDecl { name, type_name, value, line: start.line, column: start.column })
    }

    // --- ferramentas ---

    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn is_at_end(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        if !self.is_at_end() {
            self.position += 1;
        }
        token
    }

    // Consome o token atual se for igual ao esperado
    fn eat(&mut self, expected: &TokenKind) -> bool {
        if &self.current().kind == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: TokenKind, message: &str) -> Result<(), Diagnostic>{
        if self.eat(&expected) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn error(&self, message: &str) -> Diagnostic {
        self.error_code("RX100", message)
    }

    fn error_code(&self, code: &'static str, message: &str) -> Diagnostic {
        let token = self.current();
        Diagnostic::new(code, message, token.line, token.column)
    }

    // --- comandos ---

    fn parse_statement(&mut self) -> Result<Stmt, Diagnostic> {
        match self.current().kind.clone() {
            TokenKind::Show => {
                self.advance();
                let value = self.parse_expression()?;
                self.expect(TokenKind::Semicolon, "expected ';' after show")?;
                Ok(Stmt::Show(value))
            }
            TokenKind::Variable(name) => {
                let start = self.advance();

                // $x: int8 = 6;  (tipo explícito)  ou  $x = 6;  (tipo inferido)
                let declared_type = if self.eat(&TokenKind::Colon) {
                    match self.current().kind.clone() {
                        TokenKind::Identifier(type_name) => {
                            let ty = Type::from_name(&type_name).ok_or_else(|| {
                                self.error_code("RX300", &format!("unknown type '{type_name}'"))
                            })?;
                            self.advance();
                            Some(ty)
                        }
                        _ => return Err(self.error("expected a type name after ':'")),
                    }
                } else {
                    None
                };

                self.expect(TokenKind::Assign, "expected '=' after variable name")?;
                let value = self.parse_expression()?;
                self.expect(TokenKind::Semicolon, "expected ';' after assignment")?;
                Ok(Stmt::Assign { name, declared_type, value, line: start.line, column: start.column })
            }
            TokenKind::Break => {
                self.advance();
                if self.loop_depth == 0 {
                    return Err(self.error_code("RX103", "'break' is only allowed inside a loop"));
                }
                self.expect(TokenKind::Semicolon, "expected ';' after break")?;
                Ok(Stmt::Break)
            }
            TokenKind::Continue => {
                self.advance();
                if self.loop_depth == 0 {
                    return Err(self.error_code("RX103", "'continue' is only allowed inside a loop"));
                }
                self.expect(TokenKind::Semicolon, "expected ';' after continue")?;
                Ok(Stmt::Continue)
            }
            TokenKind::Stop => {
                self.advance();
                self.expect(TokenKind::Semicolon, "expected ';' after stop")?;
                Ok(Stmt::Stop)
            }

            // Todo o resto: qualquer token que não começa um comando
            _ => Err(self.error("expected a statement")),
        }
    }
        // --- expressões (uma função por nível de precedência) ---

    fn parse_expression(&mut self) -> Result<Expr, Diagnostic>  {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, Diagnostic>{
        let mut left = self.parse_sum()?;
        loop {
            let op = match self.current().kind {
                TokenKind::EqualEqual => BinaryOp::Equal,
                TokenKind::BangEqual => BinaryOp::NotEqual,
                TokenKind::Less => BinaryOp::Less,
                TokenKind::LessEqual => BinaryOp::LessEqual,
                TokenKind::Greater => BinaryOp::Greater,
                TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_sum()?;
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_sum(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_product()?;
        loop {
            let op = match self.current().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Subtract,
                _ => break,
            };
            self.advance();
            let right = self.parse_product()?;
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_product(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.current().kind {
                TokenKind::Star => BinaryOp::Multiply,
                TokenKind::Slash => BinaryOp::Divide,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        let op = match self.current().kind {
            TokenKind::Minus => UnaryOp::Negate,
            TokenKind::Bang => UnaryOp::Not,
            _ => return self.parse_primary(),
        };
        self.advance();
        let operand = self.parse_unary()?;
        Ok(Expr::Unary { op, operand: Box::new(operand) })
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        match self.current().kind.clone() {
            TokenKind::Int(n) => { self.advance(); Ok(Expr::Int(n)) }
            TokenKind::Float(n) => { self.advance(); Ok(Expr::Float(n)) }
            TokenKind::Str(s) => { self.advance(); Ok(Expr::Str(s)) }
            TokenKind::True => { self.advance(); Ok(Expr::Bool(true)) }
            TokenKind::False => { self.advance(); Ok(Expr::Bool(false)) }
            TokenKind::Variable(name) => { self.advance(); Ok(Expr::Variable(name)) }
            TokenKind::LeftParen => {
                self.advance();
                let inner = self.parse_expression()?;
                self.expect(TokenKind::RightParen, "expected ')'")?;
                Ok(inner)
            }
            TokenKind::Identifier(name) if is_uppercase_name(&name) => {
                self.advance();
                Ok(Expr::Constant(name))
            }

            _ => Err(self.error("expected an expression")),
        }
    }

}

fn is_uppercase_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}