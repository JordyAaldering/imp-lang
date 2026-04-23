use std::{collections::HashSet, mem};

use crate::{ast::*, Traverse};

pub fn dead_code_removal<'ast>(program: &mut Program<'ast, TypedAst>) {
    DeadCodeRemoval::new().trav_program(program);
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

    fn ptr<'ast>(lvis: &VarInfo<'ast, TypedAst>) -> *const () {
        lvis as *const _ as *const ()
    }
}

impl<'ast> Traverse<'ast> for DeadCodeRemoval {
    type Ast = TypedAst;

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        self.used.clear();
        self.trav_body(&mut fundef.body);
    }

    fn trav_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        self.trav_expr(assign.expr);
    }

    fn trav_body(&mut self, body: &mut Body<'ast, Self::Ast>) {
        self.trav_id(&mut body.ret);

        let mut kept_rev = Vec::with_capacity(body.stmts.len());
        for stmt in mem::take(&mut body.stmts).into_iter().rev() {
            match stmt {
                Stmt::Assign(mut assign) => {
                    if self.used.contains(&Self::ptr(assign.lhs)) {
                        self.trav_assign(&mut assign);
                        kept_rev.push(Stmt::Assign(assign));
                    }
                }
                Stmt::Printf(mut printf) => {
                    self.trav_printf(&mut printf);
                    kept_rev.push(Stmt::Printf(printf));
                }
            }
        }

        kept_rev.reverse();
        body.stmts = kept_rev;
    }

    fn trav_tensor_expr(&mut self, mut tensor: Tensor<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        let outer_used = mem::take(&mut self.used);

        if let Some(lb) = &mut tensor.lb {
            self.trav_id(lb);
        }
        self.trav_id(&mut tensor.ub);
        self.used.insert(Self::ptr(tensor.iv));

        self.trav_body(&mut tensor.body);

        self.used = outer_used;
        if let Some(lb) = &mut tensor.lb {
            self.trav_id(lb);
        }
        self.trav_id(&mut tensor.ub);

        Expr::Tensor(tensor)
    }

    fn trav_fold_expr(&mut self, mut fold: Fold<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        self.trav_id(&mut fold.neutral);

        fold.foldfun = match fold.foldfun {
            FoldFun::Name(id) => FoldFun::Name(id),
            FoldFun::Apply { id, mut args } => {
                for arg in &mut args {
                    if let FoldFunArg::Bound(bound) = arg {
                        self.trav_id(bound);
                    }
                }
                FoldFun::Apply { id, args }
            }
        };

        self.trav_tensor(&mut fold.selection);

        Expr::Fold(fold)
    }

    fn trav_array(&mut self, array: &mut Array<'ast, Self::Ast>) {
        for value in &mut array.elems {
            self.trav_id(value);
        }
    }

    fn trav_id(&mut self, id: &mut Id<'ast, Self::Ast>) {
        if let Id::Var(v) = id {
            self.used.insert(Self::ptr(*v));
        }
    }
}
