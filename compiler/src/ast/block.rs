use crate::arena::{Arena, SecondaryArena};

use super::{ArgOrVar, AstConfig, Avis, Expr};

#[derive(Clone, Debug)]
pub struct Block<Ast: AstConfig> {
    pub local_vars: Arena<Avis<Ast>>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub local_ssa: SecondaryArena<Expr>,
    pub ret: ArgOrVar,
}

// impl<Ast: AstConfig> ops::Index<ArgOrVar> for Block<Ast> {
//     type Output = Avis<Ast>;

//     fn index(&self, x: ArgOrVar) -> &Self::Output {
//         match x {
//             ArgOrVar::Arg(i) => &self.args[i],
//             ArgOrVar::Var(k) => &self.vars[k],
//             ArgOrVar::Iv(k) => &self.vars[k],
//         }
//     }
// }
