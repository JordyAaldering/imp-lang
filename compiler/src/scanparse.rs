mod lexer;
mod parser;
mod operator;
mod span;
pub(crate) mod parse_ast;

use lexer::Lexer;
use parser::Parser;
use parse_ast::Program;

pub fn scanparse(src: &str) -> Result<Program, String> {
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    parser.parse_program()
        .map_err(|e| format!("{:?}", e))
}
