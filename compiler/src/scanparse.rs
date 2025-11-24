mod lexer;
mod parse_ast;
mod parser;
mod operator;
mod span;

use lexer::Lexer;
use parse_ast::Program;
use parser::Parser;

pub fn parse(src: &str) -> Result<Program, String> {
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    parser.parse_program()
        .map_err(|e| format!("{:?}", e))
}
