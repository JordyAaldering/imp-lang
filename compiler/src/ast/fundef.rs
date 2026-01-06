use std::ops;

use slotmap::{SecondaryMap, SlotMap};

use super::{ArgOrVar, AstConfig, Avis, Expr};

#[derive(Clone, Debug)]
pub struct Fundef<Ast: AstConfig> {
    /// User-defined function name
    pub name: String,
    /// Function arguments
    pub args: Vec<Avis<Ast>>,
    /// Local identifiers
    pub ids: SlotMap<Ast::SlotKey, Avis<Ast>>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub ssa: SecondaryMap<Ast::SlotKey, Expr<Ast>>,
    /// Key of the return value
    pub ret: ArgOrVar<Ast>,
}

impl<Ast: AstConfig> ops::Index<ArgOrVar<Ast>> for Fundef<Ast> {
    type Output = Avis<Ast>;

    fn index(&self, x: ArgOrVar<Ast>) -> &Self::Output {
        match x {
            ArgOrVar::Arg(i) => &self.args[i],
            ArgOrVar::Var(k) => &self.ids[k],
            ArgOrVar::Iv(k) => &self.ids[k],
        }
    }
}
