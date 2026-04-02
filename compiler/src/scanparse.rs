mod span;
mod operator;
mod lexer;
mod parser;
pub(crate) mod parse_ast;

use lexer::Lexer;
use parser::Parser;
use parse_ast::Program as ParseAstProgram;

pub fn scanparse(src: &str) -> Result<ParseAstProgram, String> {
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    parser.parse_program()
        .map_err(|e| format!("{:?}", e))
}
