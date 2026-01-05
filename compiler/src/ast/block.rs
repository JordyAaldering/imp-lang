use crate::arena::{Arena, SecondaryArena};

use super::{ArgOrVar, AstConfig, Avis, Expr};

/// Having a seperate block is a bit finnecky, because for a tensor we want to keep track of the iv and
/// bounds in the tensor struct, but they must be defined in the block.
/// Similarly for fundefs, where args are part of the scope, but are at a seperate nesting depth
/// Probably should just remove this and inline it in the parent structs
/// Besides fundef and tensor (and in future conditionals and loops?) this should not be necessary
#[derive(Clone, Debug)]
pub struct Block<Ast: AstConfig> {
    pub ids: Arena<Avis<Ast>>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub ssa: SecondaryArena<Expr<Ast>>,
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
