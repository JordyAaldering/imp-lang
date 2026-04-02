use crate::{Rewrite, ast::*};


pub struct ConstantFold<'ast> {
    args: Vec<&'ast Avis<TypedAst>>,
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
        let l = &binary.l;
        let r = &binary.r;
        match (l, r) {
            (Id::Var(_l), Id::Var(_r)) => {
                // Todo
                Expr::Binary(binary)
            }
            _ => {
                // Nothing to do
                Expr::Binary(binary)
            }
        }
    }
}
