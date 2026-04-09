use std::{collections::HashSet, mem};

use crate::{Rewrite, ast::*};

pub fn dead_code_removal<'ast>(mut program: Program<'ast, TypedAst>) -> Program<'ast, TypedAst> {
    let mut dcr = DeadCodeRemoval::new();
    dcr.rewrite_program(&mut program);
    program
}

struct DeadCodeRemoval {
    used: HashSet<*const ()>,
}

impl DeadCodeRemoval {
    fn new() -> Self {
        Self {
            used: HashSet::new(),
        }
    }

    fn ptr<'ast>(lvis: &'ast VarInfo<'ast, TypedAst>) -> *const () {
        lvis as *const _ as *const ()
    }
}

impl<'ast> Rewrite<'ast> for DeadCodeRemoval {
    type Ast = TypedAst;

    fn rewrite_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        self.used.clear();

        let mut kept_rev = Vec::with_capacity(fundef.body.len());
        for stmt in mem::take(&mut fundef.body).into_iter().rev() {
            match stmt {
                Stmt::Return(mut ret) => {
                    self.rewrite_return(&mut ret);
                    kept_rev.push(Stmt::Return(ret));
                }
                Stmt::Assign(mut assign) => {
                    if self.used.contains(&Self::ptr(assign.lhs)) {
                        self.rewrite_assign(&mut assign);
                        kept_rev.push(Stmt::Assign(assign));
                    }
                }
            }
        }

        kept_rev.reverse();
        fundef.body = kept_rev;
        fundef.decs.retain(|lvis| self.used.contains(&Self::ptr(lvis)));
    }

    fn rewrite_cond(&mut self, mut cond: Cond<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        cond.cond = self.rewrite_id(cond.cond);
        cond.then_branch = self.rewrite_id(cond.then_branch);
        cond.else_branch = self.rewrite_id(cond.else_branch);
        Expr::Cond(cond)
    }

    fn rewrite_call(&mut self, mut call: Call<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        for arg in &mut call.args {
            *arg = self.rewrite_id(*arg);
        }
        Expr::Call(call)
    }

    fn rewrite_prf_call(&mut self, mut prf: PrfCall<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        for arg in prf.args_mut() {
            *arg = self.rewrite_id(*arg);
        }
        Expr::PrfCall(prf)
    }

    fn rewrite_tensor(&mut self, mut tensor: Tensor<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        let outer_used = mem::take(&mut self.used);

        self.rewrite_id(Id::Var(tensor.iv));
        self.rewrite_id(tensor.ret);
        self.rewrite_id(tensor.lb);
        self.rewrite_id(tensor.ub);

        let mut kept_rev = Vec::with_capacity(tensor.body.len());
        for stmt in mem::take(&mut tensor.body).into_iter().rev() {
            match stmt {
                Stmt::Return(mut ret) => {
                    self.rewrite_return(&mut ret);
                    kept_rev.push(Stmt::Return(ret));
                }
                Stmt::Assign(mut assign) => {
                    if self.used.contains(&Self::ptr(assign.lhs)) {
                        self.rewrite_assign(&mut assign);
                        kept_rev.push(Stmt::Assign(assign));
                    }
                }
            }
        }
        kept_rev.reverse();
        tensor.body = kept_rev;

        self.used = outer_used;
        tensor.lb = self.rewrite_id(tensor.lb);
        tensor.ub = self.rewrite_id(tensor.ub);
        Expr::Tensor(tensor)
    }

    fn rewrite_fold(&mut self, mut fold: Fold<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        fold.neutral = self.rewrite_id(fold.neutral);

        fold.foldfun = match fold.foldfun {
            FoldFun::Name(id) => FoldFun::Name(id),
            FoldFun::Apply { id, mut args } => {
                for arg in &mut args {
                    if let FoldFunArg::Bound(bound) = arg {
                        *bound = self.rewrite_id(bound.clone());
                    }
                }
                FoldFun::Apply { id, args }
            }
        };

        fold.selection = match self.rewrite_tensor(fold.selection) {
            Expr::Tensor(tensor) => tensor,
            _ => unreachable!("rewrite_tensor must return Tensor"),
        };

        Expr::Fold(fold)
    }

    fn rewrite_array(&mut self, mut array: Array<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        for value in &mut array.elems {
            *value = self.rewrite_id(*value);
        }
        Expr::Array(array)
    }

    fn rewrite_id(&mut self, id: Id<'ast, Self::Ast>) -> Id<'ast, Self::Ast> {
        if let Id::Var(v) = &id {
            self.used.insert(Self::ptr(v));
        }
        id
    }
}
