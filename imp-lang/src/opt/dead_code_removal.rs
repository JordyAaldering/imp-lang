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
        self.rewrite_body(&mut fundef.body);
        fundef.decs.retain(|lvis| self.used.contains(&Self::ptr(lvis)));
    }

    fn rewrite_body(&mut self, body: &mut Body<'ast, Self::Ast>) {
        self.rewrite_id(body.ret);

        let mut kept_rev = Vec::with_capacity(body.stmts.len());
        for stmt in mem::take(&mut body.stmts).into_iter().rev() {
            match stmt {
                Stmt::Assign(mut assign) => {
                    if self.used.contains(&Self::ptr(assign.lhs)) {
                        self.rewrite_assign(&mut assign);
                        kept_rev.push(Stmt::Assign(assign));
                    }
                }
            }
        }

        kept_rev.reverse();
        body.stmts = kept_rev;
    }

    fn rewrite_cond(&mut self, mut cond: Cond<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        cond.cond = self.rewrite_id(cond.cond);
        self.rewrite_body(&mut cond.then_branch);
        self.rewrite_body(&mut cond.else_branch);
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

        if let Some(lb) = tensor.lb {
            self.rewrite_id(lb);
        }
        self.rewrite_id(tensor.ub);
        self.rewrite_id(Id::Var(tensor.iv));

        self.rewrite_body(&mut tensor.body);

        self.used = outer_used;
        if let Some(lb) = &mut tensor.lb {
            *lb = self.rewrite_id(*lb);
        }
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
