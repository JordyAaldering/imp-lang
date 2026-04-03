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
                    if self.used.contains(&Self::ptr(assign.lvis)) {
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

    fn rewrite_tensor(&mut self, mut tensor: Tensor<'ast, Self::Ast>) -> Tensor<'ast, Self::Ast> {
        let outer_used = mem::take(&mut self.used);

        self.rewrite_id(Id::Var(tensor.iv));
        self.rewrite_id(tensor.ret.clone());
        self.rewrite_id(tensor.lb.clone());
        self.rewrite_id(tensor.ub.clone());

        let mut kept_rev = Vec::with_capacity(tensor.body.len());
        for stmt in mem::take(&mut tensor.body).into_iter().rev() {
            match stmt {
                Stmt::Return(mut ret) => {
                    self.rewrite_return(&mut ret);
                    kept_rev.push(Stmt::Return(ret));
                }
                Stmt::Assign(mut assign) => {
                    if self.used.contains(&Self::ptr(assign.lvis)) {
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
        tensor
    }

    fn rewrite_binary(&mut self, mut binary: Binary<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        binary.l = self.rewrite_id(binary.l);
        binary.r = self.rewrite_id(binary.r);
        Expr::Binary(binary)
    }

    fn rewrite_unary(&mut self, mut unary: Unary<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        unary.r = self.rewrite_id(unary.r);
        Expr::Unary(unary)
    }

    fn rewrite_array(&mut self, mut array: Array<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        for value in &mut array.values {
            *value = self.rewrite_id(value.clone());
        }
        Expr::Array(array)
    }

    fn rewrite_sel(&mut self, mut sel: Sel<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        sel.arr = self.rewrite_id(sel.arr);
        sel.idx = self.rewrite_id(sel.idx);
        Expr::Sel(sel)
    }

    fn rewrite_id(&mut self, id: Id<'ast, Self::Ast>) -> Id<'ast, Self::Ast> {
        if let Id::Var(v) = &id {
            self.used.insert(Self::ptr(v));
        }
        id
    }
}
