use std::collections::{HashMap, HashSet};

use super::*;

#[derive(Clone, Debug)]
pub struct Program<'ast, Ast: AstConfig> {
    /// Top-level free functions. Names are unique; direct overloading is disallowed.
    pub functions: HashMap<String, Fundef<'ast, Ast>>,
    /// Type-set declarations (for example: `typeset Num;`).
    pub typesets: HashSet<String>,
    /// Type-set membership declarations (for example: `member Num :: u32;`).
    pub members: HashMap<String, Vec<BaseType>>,
    /// Surface trait declarations. These are parsed and preserved before trait resolution exists.
    pub traits: HashMap<String, TraitDef>,
    /// Surface impl declarations. These are parsed and preserved before impl lowering exists.
    pub impls: Vec<ImplDef>,
}
