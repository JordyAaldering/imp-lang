use crate::{Rewrite, ast::*};

pub fn constant_fold<'ast>(mut program: Program<'ast, TypedAst>) -> Program<'ast, TypedAst> {
    let mut cf = ConstantFold::new();
    cf.rewrite_program(&mut program);
    program
}

pub struct ConstantFold;

impl ConstantFold {
    pub fn new() -> Self {
        Self
    }

    fn fold_expr<'ast>(&self, expr: Expr<'ast, TypedAst>, prefix: &[Stmt<'ast, TypedAst>]) -> Expr<'ast, TypedAst> {
        match expr {
            Expr::Tensor(mut tensor) => {
                // Tensor scope has its own sequential body.
                for i in 0..tensor.body.len() {
                    let (head, tail) = tensor.body.split_at_mut(i);
                    let stmt = &mut tail[0];
                    self.fold_stmt(stmt, head);
                }
                Expr::Tensor(tensor)
            }
            Expr::Binary(binary) => self.fold_binary(binary, prefix),
            Expr::Unary(unary) => Expr::Unary(unary),
            Expr::Id(id) => Expr::Id(id),
            Expr::Bool(v) => Expr::Bool(v),
            Expr::U32(v) => Expr::U32(v),
        }
    }

    fn fold_stmt<'ast>(&self, stmt: &mut Stmt<'ast, TypedAst>, prefix: &[Stmt<'ast, TypedAst>]) {
        match stmt {
            Stmt::Assign(assign) => {
                let folded = self.fold_expr((*assign.expr).clone(), prefix);
                assign.expr = Box::leak(Box::new(folded));
            }
            Stmt::Return(_) => {}
        }
    }

    fn fold_binary<'ast>(&self, binary: Binary<'ast, TypedAst>, prefix: &[Stmt<'ast, TypedAst>]) -> Expr<'ast, TypedAst> {
        if matches!(binary.op, Bop::Add) {
            let l = self.const_u32_of_id(&binary.l, prefix);
            let r = self.const_u32_of_id(&binary.r, prefix);
            if let (Some(l), Some(r)) = (l, r) {
                return Expr::U32(l + r);
            }
        }
        Expr::Binary(binary)
    }

    fn const_u32_of_id<'ast>(&self, id: &Id<'ast, TypedAst>, prefix: &[Stmt<'ast, TypedAst>]) -> Option<u32> {
        match id {
            Id::Arg(_) => None,
            Id::Var(target) => {
                for stmt in prefix.iter().rev() {
                    if let Stmt::Assign(assign) = stmt {
                        if std::ptr::eq(assign.lvis, *target) {
                            return match assign.expr {
                                Expr::U32(v) => Some(*v),
                                _ => None,
                            };
                        }
                    }
                }
                None
            }
        }
    }
}

impl<'ast> Rewrite<'ast> for ConstantFold {
    type Ast = TypedAst;

    fn rewrite_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        for i in 0..fundef.body.len() {
            let (head, tail) = fundef.body.split_at_mut(i);
            let stmt = &mut tail[0];
            self.fold_stmt(stmt, head);
        }
    }
}
