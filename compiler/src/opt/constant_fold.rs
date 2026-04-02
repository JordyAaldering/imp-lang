use crate::{Rewrite, ast::*};

pub fn constant_fold<'ast>(program: Program<'ast, TypedAst>) -> Program<'ast, TypedAst> {
    let mut cf = ConstantFold::new();
    cf.rewrite_program(program)
}

pub struct ConstantFold<'ast> {
    args: Vec<&'ast Farg<TypedAst>>,
}

impl<'ast> ConstantFold<'ast> {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }
}

impl<'ast> Rewrite<'ast> for ConstantFold<'ast> {
    type Ast = TypedAst;

    fn rewrite_fundef(&mut self, fundef: Fundef<'ast, Self::Ast>) -> Fundef<'ast, Self::Ast> {
        self.args = fundef.args.clone();
        let body = fundef.body.into_iter().map(|s| self.rewrite_stmt(s)).collect();
        self.args.clear();
        Fundef { body, ..fundef }
    }

    fn rewrite_binary(&mut self, binary: Binary<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        if matches!(binary.op, Bop::Add) {
            let l = match &binary.l {
                Id::Var(lvis) => match lvis.ssa {
                    Some(Expr::U32(v)) => Some(v),
                    _ => None,
                },
                Id::Arg(_) => None,
            };

            let r = match &binary.r {
                Id::Var(lvis) => match lvis.ssa {
                    Some(Expr::U32(v)) => Some(v),
                    _ => None,
                },
                Id::Arg(_) => None,
            };

            if let (Some(l), Some(r)) = (l, r) {
                return Expr::U32(l + r);
            }
        }

        Expr::Binary(binary)
    }
}
