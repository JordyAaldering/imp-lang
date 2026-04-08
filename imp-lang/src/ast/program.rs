use std::collections::HashMap;

use super::*;

#[derive(Clone, Debug)]
pub struct Program<'ast, Ast: AstConfig> {
    /// Top-level free functions indexed by internal keys.
    pub functions: HashMap<String, Fundef<'ast, Ast>>,
}
