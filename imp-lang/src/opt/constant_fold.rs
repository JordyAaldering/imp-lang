use std::collections::HashMap;

use crate::{ast::*, Rewrite};

pub fn constant_fold<'ast>(mut program: Program<'ast, TypedAst>) -> Program<'ast, TypedAst> {
    let mut cf = ConstantFold::new();
    cf.rewrite_program(&mut program);
    program
}

pub struct ConstantFold {
    known: HashMap<*const (), u32>,
}

impl ConstantFold {
    pub fn new() -> Self {
        Self {
            known: HashMap::new(),
        }
    }

    fn ptr<'ast>(lvis: &'ast VarInfo<'ast, TypedAst>) -> *const () {
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
        for stmt in &mut fundef.body {
            self.rewrite_stmt(stmt);
        }
    }

    fn rewrite_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        let new_expr = self.rewrite_expr((*assign.expr).clone());

        match &new_expr {
            Expr::U32(v) => { self.known.insert(Self::ptr(assign.lvis), *v); }
            _ => { self.known.remove(&Self::ptr(assign.lvis)); }
        }

        assign.expr = Box::leak(Box::new(new_expr));
    }

    fn rewrite_binary(&mut self, binary: Binary<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        if matches!(binary.op, Bop::Add) {
            if let (Some(l), Some(r)) = (self.const_u32(&binary.l), self.const_u32(&binary.r)) {
                return Expr::U32(l + r);
            }
        }
        Expr::Binary(binary)
    }

    fn rewrite_tensor(&mut self, mut tensor: Tensor<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        for stmt in &mut tensor.body {
            self.rewrite_stmt(stmt);
        }
        Expr::Tensor(tensor)
    }
}
