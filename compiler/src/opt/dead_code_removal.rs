use crate::{Rewrite, ast::*};
use std::collections::HashSet;
use std::mem;

pub fn dead_code_removal<'ast>(mut program: Program<'ast, TypedAst>) -> Program<'ast, TypedAst> {
    let mut dcr = DCR::new();
    dcr.rewrite_program(&mut program);
    program
}

struct DCR {
    used: HashSet<*const ()>,
}

impl DCR {
    fn new() -> Self {
        Self {
            used: HashSet::new(),
        }
    }

    fn ptr<'ast>(lvis: &'ast Lvis<'ast, TypedAst>) -> *const () {
        lvis as *const _ as *const ()
    }
}

impl<'ast> Rewrite<'ast> for DCR {
    type Ast = TypedAst;

    // Visiting an id means it is used — mark it.
    fn rewrite_id(&mut self, id: Id<'ast, Self::Ast>) -> Id<'ast, Self::Ast> {
        if let Id::Var(v) = &id {
            self.used.insert(Self::ptr(v));
        }
        id
    }

    // Recurse into binary operands so rewrite_id is called on each.
    fn rewrite_binary(&mut self, binary: Binary<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        self.rewrite_id(binary.l.clone());
        self.rewrite_id(binary.r.clone());
        Expr::Binary(binary)
    }

    // Recurse into unary operand so rewrite_id is called on it.
    fn rewrite_unary(&mut self, unary: Unary<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        self.rewrite_id(unary.r.clone());
        Expr::Unary(unary)
    }

    fn rewrite_return(&mut self, ret: &mut Return<'ast, Self::Ast>) {
        self.rewrite_id(ret.id.clone());
    }

    fn rewrite_tensor(&mut self, mut tensor: Tensor<'ast, Self::Ast>) -> Tensor<'ast, Self::Ast> {
        // Switch to a fresh inner scope for the tensor body.
        let outer_used = mem::take(&mut self.used);

        // iv and ret seed the inner backward sweep.
        self.rewrite_id(Id::Var(tensor.iv));
        self.rewrite_id(tensor.ret.clone());

        let mut kept_rev = Vec::with_capacity(tensor.body.len());
        for stmt in mem::take(&mut tensor.body).into_iter().rev() {
            match stmt {
                Stmt::Return(mut ret) => {
                    self.rewrite_return(&mut ret);
                    kept_rev.push(Stmt::Return(ret));
                }
                Stmt::Assign(mut assign) => {
                    if self.used.contains(&Self::ptr(assign.lvis)) {
                        // rewrite_assign (default) calls rewrite_expr, which dispatches
                        // to our overridden rewrite_binary/rewrite_unary/rewrite_id,
                        // marking all transitive deps as used.
                        self.rewrite_assign(&mut assign);
                        kept_rev.push(Stmt::Assign(assign));
                    }
                }
            }
        }
        kept_rev.reverse();
        tensor.body = kept_rev;

        // Restore outer scope, then mark lb/ub as used there (they come from
        // the outer scope and are the tensor's external dependencies).
        self.used = outer_used;
        self.rewrite_id(tensor.lb.clone());
        self.rewrite_id(tensor.ub.clone());
        tensor
    }

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
}
