//! # Scanning-parsing (`scp`)
mod span;
mod operator;
mod lexer;
mod parser;

use lexer::Lexer;
use parser::Parser;
use crate::ast::{Program, ParsedAst};

pub fn scanparse(src: &str) -> Result<Program<'static, ParsedAst>, String> {
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    parser.parse_program()
        .map_err(|e| format!("{:?}", e))
}
