use super::*;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Program<'ast, Ast: AstConfig> {
    /// Function wrappers grouped by name, each potentially containing multiple overloads.
    /// During parsing, functions with the same name are automatically grouped into a single wrapper.
    pub fundefs: HashMap<String, FundefWrapper<'ast, Ast>>,
}
