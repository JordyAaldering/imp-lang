use std::ops;

use slotmap::{DefaultKey, SecondaryMap, SlotMap};

use crate::ast::Expr;

use super::{ArgOrVar, AstConfig, Avis};

#[derive(Clone, Debug)]
pub struct Fundef<Ast: AstConfig> {
    /// User-defined function name
    pub name: String,
    /// Function arguments
    pub args: Vec<Avis<Ast>>,
    /// Local identifiers
    pub ids: SlotMap<DefaultKey, Avis<Ast>>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub ssa: SecondaryMap<DefaultKey, Expr<Ast>>,
    /// Key of the return value
    pub ret: ArgOrVar,
}

impl<Ast: AstConfig> ops::Index<ArgOrVar> for Fundef<Ast> {
    type Output = Avis<Ast>;

    fn index(&self, x: ArgOrVar) -> &Self::Output {
        match x {
            ArgOrVar::Arg(i) => &self.args[i],
            ArgOrVar::Var(k) => &self.ids[k],
            ArgOrVar::Iv(k) => &self.ids[k],
        }
    }
}
