//! # Scanning-parsing (`scp`)
mod span;
mod operator;
mod lexer;
mod parser;

use lexer::Lexer;
use parser::Parser;
use crate::ast::{Scope, Program, ParsedAst};

pub fn scanparse<'ast>(src: &str, scope: &'ast Scope<'ast, ParsedAst>) -> Result<Program<'ast, ParsedAst>, String> {
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer, scope);
    parser.parse_program()
        .map_err(|e| format!("{:?}", e))
}
