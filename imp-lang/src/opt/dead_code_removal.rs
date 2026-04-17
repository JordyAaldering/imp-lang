use std::{collections::HashSet, mem};
use typed_arena::Arena;

use crate::{Rewrite, ast::*};

pub fn dead_code_removal<'ast>(mut program: Program<'ast, TypedAst>) -> Program<'ast, TypedAst> {
    let mut dcr = DeadCodeRemoval::new();
    dcr.rewrite_program(&mut program);
    program
}

struct DeadCodeRemoval {
    used: HashSet<*const ()>,
    expr_arena: *const (),
}

impl DeadCodeRemoval {
    fn new() -> Self {
        Self {
            used: HashSet::new(),
            expr_arena: std::ptr::null(),
        }
    }

    fn set_expr_arena<'ast>(&mut self, arena: &Arena<Expr<'ast, TypedAst>>) {
        self.expr_arena = arena as *const _ as *const ();
    }

    fn alloc_expr_in_arena<'ast>(&self, expr: Expr<'ast, TypedAst>) -> &'ast Expr<'ast, TypedAst> {
        let arena = unsafe { &*(self.expr_arena as *const Arena<Expr<'ast, TypedAst>>) };
        // SAFETY: arena belongs to current fundef and outlives produced expr references.
        unsafe { std::mem::transmute(arena.alloc(expr)) }
    }

    fn ptr<'ast>(lvis: &VarInfo<'ast, TypedAst>) -> *const () {
        lvis as *const _ as *const ()
    }
}

impl<'ast> Rewrite<'ast> for DeadCodeRemoval {
    type Ast = TypedAst;

    fn rewrite_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        self.used.clear();
        self.set_expr_arena(&fundef.exprs);
        self.rewrite_body(&mut fundef.body);
    }

    fn rewrite_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        let new_expr = self.rewrite_expr((*assign.expr).clone());
        assign.expr = self.alloc_expr_in_arena(new_expr);
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
                Stmt::Printf(mut printf) => {
                    self.rewrite_printf(&mut printf);
                    kept_rev.push(Stmt::Printf(printf));
                }
            }
        }

        kept_rev.reverse();
        body.stmts = kept_rev;
    }

    fn rewrite_printf(&mut self, printf: &mut Printf<'ast, Self::Ast>) {
        printf.id = self.rewrite_id(printf.id);
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
            self.used.insert(Self::ptr(*v));
        }
        id
    }
}
