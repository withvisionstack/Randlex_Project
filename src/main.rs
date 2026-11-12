#![allow(dead_code)]

mod ast;
mod diagnostic;
mod interpreter;
mod lexer;
mod parser;
mod token;
mod value;
mod types;
mod checker;

use diagnostic::{explain, Diagnostic};
use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;

fn fail(diagnostic: &Diagnostic, file: &str, source: &str, development: bool) -> ! {
    eprint!("{}", diagnostic.render(file, source, development));
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // rendlex --explain RX001
    if let Some(i) = args.iter().position(|a| a == "--explain") {
        match args.get(i + 1).and_then(|code| explain(code)) {
            Some(text) => println!("{text}"),
            None => eprintln!("unknown error code"),
        }
        return;
    }

    let development = args.iter().any(|a| a == "--development");
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("usage: rendlex <file.rdx> [--development]");
        std::process::exit(2);
    };
    let source = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("could not read {path}: {e}");
        std::process::exit(2);
    });

    let tokens = Lexer::new(&source)
        .tokenize()
        .unwrap_or_else(|d| fail(&d, path, &source, development));
    let program = Parser::new(tokens)
        .parse_program()
        .unwrap_or_else(|d| fail(&d, path, &source, development));
    checker::Checker::new()
        .check(&program)
        .unwrap_or_else(|d| fail(&d, path, &source, development));

    if let Err(e) = Interpreter::new().run(&program) {
        eprintln!("error[RX200]: {}", e.message);
        std::process::exit(1);
    }
}