use crate::diagnostic::Diagnostic;
use crate::token::{Token, TokenKind};
use std::{iter::Peekable, str::Chars};

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { chars: source.chars().peekable(), line: 1, column: 1 }
    }

    // --- ferramentas básicas ---

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn peek_next(&self) -> Option<char> {
        let mut copy = self.chars.clone();
        copy.next();
        copy.next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    // --- laço principal ---

    pub fn tokenize(mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            let (line, column) = (self.line, self.column);
            let Some(c) = self.advance() else {
                tokens.push(Token { kind: TokenKind::Eof, line, column });
                return Ok(tokens);
            };

            let kind = match c {
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                '{' => TokenKind::LeftBrace,
                '}' => TokenKind::RightBrace,
                '[' => TokenKind::LeftBracket,
                ']' => TokenKind::RightBracket,
                ',' => TokenKind::Comma,
                ';' => TokenKind::Semicolon,
                ':' => TokenKind::Colon,
                '.' => TokenKind::Dot,
                '=' => if self.match_char('=') { TokenKind::EqualEqual } else { TokenKind::Assign },
                '!' => if self.match_char('=') { TokenKind::BangEqual } else { TokenKind::Bang },
                '<' => if self.match_char('=') { TokenKind::LessEqual } else { TokenKind::Less },
                '>' => if self.match_char('=') { TokenKind::GreaterEqual } else { TokenKind::Greater },
                '"' => self.read_string(line, column)?,
                '$' => self.read_variable(line, column)?,
                c if c.is_ascii_digit() => self.read_number(c),
                c if is_identifier_start(c) => self.read_word(c),
                other => {
                    let mut diagnostic = Diagnostic::new(
                        "RX001",
                        format!("unexpected character '{other}'"),
                        line,
                        column,
                    );
                    if other == '\\' {
                        diagnostic = diagnostic
                            .help("'\\' is only valid inside a string, like \"a\\nb\"")
                            .help("remove '\\'");
                    }
                    return Err(diagnostic);
                }
            };

            tokens.push(Token { kind, line, column });
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_next() == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    // --- leitores de tokens maiores ---

    fn read_number(&mut self, first: char) -> TokenKind {
        let mut text = String::from(first);
        while let Some(c) = self.peek().filter(|c| c.is_ascii_digit()) {
            text.push(c);
            self.advance();
        }
        if self.peek() == Some('.') && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            text.push('.');
            self.advance();
            while let Some(c) = self.peek().filter(|c| c.is_ascii_digit()) {
                text.push(c);
                self.advance();
            }
            return TokenKind::Float(text.parse().unwrap());
        }
        match text.parse::<i64>() {
            Ok(n) => TokenKind::Int(n),
            Err(_) => TokenKind::Float(text.parse().unwrap()),
        }
    }

    fn read_word(&mut self, first: char) -> TokenKind {
        let mut text = String::from(first);
        while let Some(c) = self.peek().filter(|c| is_identifier_part(*c)) {
            text.push(c);
            self.advance();
        }
        match text.as_str() {
            "Show" => TokenKind::Show,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "const" => TokenKind::Const,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "stop" => TokenKind::Stop,
            _ => TokenKind::Identifier(text),
        }
    }

    fn read_variable(&mut self, line: usize, column: usize) -> Result<TokenKind, Diagnostic> {
        match self.peek() {
            Some(c) if is_identifier_start(c) => {
                let mut name = String::new();
                while let Some(c) = self.peek().filter(|c| is_identifier_part(*c)) {
                    name.push(c);
                    self.advance();
                }
                Ok(TokenKind::Variable(name))
            }
            _ => Err(Diagnostic::new("RX004", "expected a variable name after '$'", line, column)),
        }
    }

    fn read_string(&mut self, line: usize, column: usize) -> Result<TokenKind, Diagnostic> {
        let mut text = String::new();
        loop {
            match self.advance() {
                None => {
                    return Err(Diagnostic::new("RX002", "unterminated string", line, column)
                        .help("add a closing '\"'"));
                }
                Some('"') => return Ok(TokenKind::Str(text.into())),
                Some('\\') => match self.advance() {
                    Some('n') => text.push('\n'),
                    Some('t') => text.push('\t'),
                    Some('"') => text.push('"'),
                    Some('\\') => text.push('\\'),
                    _ => {
                        return Err(Diagnostic::new(
                            "RX003",
                            "invalid escape sequence",
                            self.line,
                            self.column,
                        )
                            .help("valid escapes: \\n \\t \\\" \\\\"));
                    }
                },
                Some(c) => text.push(c),
            }
        }
    }
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_identifier_part(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind::*;

    fn kinds(src: &str) -> Vec<crate::token::TokenKind> {
        Lexer::new(src).tokenize().unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn show_soma() {
        assert_eq!(kinds("Show 1 + 2;"), vec![Show, Int(1), Plus, Int(2), Semicolon, Eof]);
    }

    #[test]
    fn variavel_e_float() {
        assert_eq!(kinds("$a = 1.5;"), vec![Variable("a".into()), Assign, Float(1.5), Semicolon, Eof]);
    }

    #[test]
    fn string_com_escape() {
        assert_eq!(kinds(r#""a\nb""#), vec![Str("a\nb".into()), Eof]);
    }
}