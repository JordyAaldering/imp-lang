use std::{collections::HashSet, mem};

use crate::ast::*;

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

    type ExprOut = ();

    const EXPR_DEFAULT: Self::ExprOut = ();

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        self.used.clear();
        self.trav_body(&mut fundef.body);

        let mut kept_rev = Vec::with_capacity(fundef.shape_prelude.len());
        for mut assign in mem::take(&mut fundef.shape_prelude).into_iter().rev() {
            if self.used.contains(&Self::ptr(assign.lhs)) {
                self.trav_assign(&mut assign);
                kept_rev.push(assign);
            }
        }

        kept_rev.reverse();
        fundef.shape_prelude = kept_rev;
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

    fn trav_tensor(&mut self, tensor: &mut Tensor<'ast, Self::Ast>) {
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
    }

    fn trav_fold(&mut self, fold: &mut Fold<'ast, Self::Ast>) {
        self.trav_id(&mut fold.neutral);

        if let FoldFun::Apply { args, .. } = &mut fold.foldfun {
            for arg in args {
                if let FoldFunArg::Bound(bound) = arg {
                    self.trav_id(bound);
                }
            }
        }

        self.trav_tensor(&mut fold.selection);
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
