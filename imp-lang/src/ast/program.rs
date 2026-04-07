use super::*;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Program<'ast, Ast: AstConfig> {
    /// Top-level free functions. Names are unique; direct overloading is disallowed.
    pub functions: HashMap<String, Fundef<'ast, Ast>>,
    /// Generic free functions, to be monomorphized before code generation.
    pub generic_functions: HashMap<String, GenericFundef<'ast, Ast>>,
    /// Type-set declarations (for example: `type Num :: T;`).
    pub typesets: HashMap<String, TypeSetDef>,
    /// Type-set membership declarations (for example: `member Num :: u32;`).
    pub members: Vec<MemberDef>,
    /// Surface trait declarations. These are parsed and preserved before trait resolution exists.
    pub traits: HashMap<String, TraitDef>,
    /// Surface impl declarations. These are parsed and preserved before impl lowering exists.
    pub impls: Vec<ImplDef>,
}
