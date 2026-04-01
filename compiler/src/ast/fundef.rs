use super::{ArgOrVar, AstConfig, Avis, Expr};

/// Maybe the whole thing is just overkill, and we should instead just use a refcell tree
/// structure to store the ast, which also allows us to lookup parent nodes.
/// https://github.com/0xSaksham/tree_data_structure
/// The main issue is likely in modifying parent nodes, but
///   we should never modify parent nodes, only ourselves
///   the only exception is maybe to adjust their declaration may (which is scope-local),
///   but maybe that should still be done by the fundef after the fact, by storing changes in the
///   knapsack.
///
///   traversals should then always return a new node (possibly unchanged)
///   to force (eg) fundefs to adjust their map of declarations
///
/// But how to link from this declarations map to the corresponding nodes?
/// Can the reference be used as a key?
#[derive(Clone, Debug)]
pub struct Fundef<'ast, Ast: AstConfig> {
    /// User-defined function name
    pub name: String,
    /// Function arguments
    pub args: Vec<&'ast Avis<'ast, Ast>>,
    /// Local identifiers
    pub ids: Vec<&'ast Avis<'ast, Ast>>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub ssa: Vec<(&'ast Avis<'ast, Ast>, &'ast Expr<'ast, Ast>)>,
    /// Key of the return value
    pub ret: ArgOrVar<'ast, Ast>,
}

impl<'ast, Ast: AstConfig> Fundef<'ast, Ast> {
    pub fn nameof(&self, k: ArgOrVar<'ast, Ast>) -> &str {
        match k {
            ArgOrVar::Arg(i) => &self.args[i].name,
            ArgOrVar::Var(v) => &v.name,
            ArgOrVar::Iv(v) => &v.name,
        }
    }

    pub fn typof(&self, k: ArgOrVar<'ast, Ast>) -> &Ast::ValueType {
        match k {
            ArgOrVar::Arg(i) => &self.args[i].ty,
            ArgOrVar::Var(v) => &v.ty,
            ArgOrVar::Iv(v) => &v.ty,
        }
    }

    pub fn find_ssa(&self, key: &'ast Avis<'ast, Ast>) -> Option<&'ast Expr<'ast, Ast>> {
        self.ssa.iter().find_map(|(id, expr)| if std::ptr::eq(*id, key) { Some(*expr) } else { None })
    }
}
