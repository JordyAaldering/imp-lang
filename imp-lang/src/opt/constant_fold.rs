use std::collections::HashMap;
use typed_arena::Arena;

use crate::{ast::*, Rewrite};

pub fn constant_fold<'ast>(mut program: Program<'ast, TypedAst>) -> Program<'ast, TypedAst> {
    let mut cf = ConstantFold::new();
    cf.rewrite_program(&mut program);
    program
}

pub struct ConstantFold {
    known: HashMap<*const (), u32>,
    expr_arena: *const (),
}

impl ConstantFold {
    pub fn new() -> Self {
        Self {
            known: HashMap::new(),
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

    fn const_u32<'ast>(&self, id: &Id<'ast, TypedAst>) -> Option<u32> {
        match id {
            Id::Var(lvis) => self.known.get(&Self::ptr(lvis)).copied(),
            Id::Arg(_) => None,
        }
    }
}

impl<'ast> Rewrite<'ast> for ConstantFold {
    type Ast = TypedAst;

    fn rewrite_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        self.set_expr_arena(&fundef.exprs);
        self.rewrite_body(&mut fundef.body);
    }

    fn rewrite_body(&mut self, body: &mut Body<'ast, Self::Ast>) {
        for stmt in &mut body.stmts {
            self.rewrite_stmt(stmt);
        }
        body.ret = self.rewrite_id(body.ret);
    }

    fn rewrite_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        let new_expr = self.rewrite_expr((*assign.expr).clone());

        match &new_expr {
            Expr::Const(Const::U32(v)) => {
                self.known.insert(Self::ptr(assign.lhs), *v);
                assign.expr = self.alloc_expr_in_arena(new_expr);
            }
            _ => {
                debug_assert!(!self.known.contains_key(&Self::ptr(assign.lhs)));
            }
        }
    }

    fn rewrite_prf_call(&mut self, prf_call: PrfCall<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        use PrfCall::*;
        match &prf_call {
            AddSxS(l, r) => {
                if let (Some(l), Some(r)) = (self.const_u32(l), self.const_u32(r)) {
                    return Expr::Const(Const::U32(l + r));
                }
            }
            SubSxS(l, r) => {
                if let (Some(l), Some(r)) = (self.const_u32(l), self.const_u32(r)) {
                    return Expr::Const(Const::U32(l - r));
                }
            }
            MulSxS(l, r) => {
                if let (Some(l), Some(r)) = (self.const_u32(l), self.const_u32(r)) {
                    return Expr::Const(Const::U32(l * r));
                }
            }
            DivSxS(l, r) => {
                if let (Some(l), Some(r)) = (self.const_u32(l), self.const_u32(r)) && r != 0 {
                    return Expr::Const(Const::U32(l / r));
                }
            }
            _ => (),
        }

        Expr::PrfCall(prf_call)
    }
}
