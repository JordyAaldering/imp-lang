use std::collections::HashMap;

use super::*;

#[derive(Clone, Debug)]
pub struct Program<'ast, Ast: AstConfig> {
    pub functions: HashMap<String, Fundef<'ast, Ast>>,
}
