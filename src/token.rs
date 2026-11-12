use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literais
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    // Nomes
    Variable(String),   // $nome (guarda sem o $)
    Identifier(String), // echo não, isso é palavra-chave; aqui entram text, order, etc.
    // Palavras-chave
    Show, If, Else, While, Function, Return, True, False,
    Const, Break, Continue, Stop,
    // Operadores
    Plus, Minus, Star, Slash,
    Assign, EqualEqual, Bang, BangEqual,
    Less, LessEqual, Greater, GreaterEqual,
    // Pontuação
    LeftParen, RightParen, LeftBrace, RightBrace, LeftBracket, RightBracket,
    Comma, Semicolon, Colon, Dot,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}