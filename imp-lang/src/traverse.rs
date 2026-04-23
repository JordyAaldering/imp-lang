use crate::ast::*;

pub trait Traverse<'ast> {
    type Ast: AstConfig + 'ast;

    // Declarations

    fn trav_program(&mut self, program: &mut Program<'ast, Self::Ast>) {
        for (_, groups) in &mut program.overloads {
            for (_, fundefs) in groups {
                for fundef in fundefs {
                    let mut fundef = fundef.borrow_mut();
                    self.trav_fundef(&mut fundef);
                }
            }
        }
    }

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        self.trav_fargs(&mut fundef.args);

        for vardec in fundef.decs.iter_mut() {
            self.trav_vardec(vardec);
        }

        for assign in &mut fundef.shape_prelude {
            self.trav_assign(assign);
        }

        self.trav_body(&mut fundef.body);
    }

    fn trav_fargs(&mut self, args: &mut [Farg]) {
        for arg in args {
            self.trav_farg(arg);
        }
    }

    fn trav_farg(&mut self, _arg: &mut Farg) {}

    fn trav_vardec(&mut self, _vardec: &mut VarInfo<'ast, Self::Ast>) {}

    // Statements

    fn trav_body(&mut self, body: &mut Body<'ast, Self::Ast>) {
        for stmt in &mut body.stmts {
            self.trav_stmt(stmt);
        }

        Self::Ast::trav_operand(self, &mut body.ret);
    }

    fn trav_stmt(&mut self, stmt: &mut Stmt<'ast, Self::Ast>) {
        use Stmt::*;
        match stmt {
            Assign(n) => self.trav_assign(n),
            Printf(n) => self.trav_printf(n),
        }
    }

    fn trav_assign(&mut self, mut assign: &mut Assign<'ast, Self::Ast>) {
        assign.expr = self.trav_expr(assign.expr);
    }
``
    fn trav_printf(&mut self, printf: &mut Printf<'ast, Self::Ast>) {
        self.trav_id(&mut printf.id);
    }

    // Expressions

    fn trav_expr(&mut self, expr: Expr<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        use Expr::*;
        match expr {
            Cond(n) => self.trav_cond(n),
            Call(n) => self.trav_call(n),
            PrfCall(n) => self.trav_prf(n),
            Tensor(n) => self.trav_tensor(n),
            Fold(n) => self.trav_fold(n),
            Array(n) => self.trav_array(n),
            Id(mut n) => {
                self.trav_id(&mut n);
                Expr::Id(n)
            }
            Const(mut n) => {
                self.trav_const(&mut n);
                Expr::Const(n)
            }
        }
    }

    fn trav_cond(&mut self, mut cond: Cond<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        Self::Ast::trav_operand(self, &mut cond.cond);
        self.trav_body(&mut cond.then_branch);
        self.trav_body(&mut cond.else_branch);
        Expr::Cond(cond)
    }

    fn trav_call(&mut self, mut call: Call<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        for arg in &mut call.args {
            Self::Ast::trav_operand(self, arg);
        }
        Expr::Call(call)
    }

    fn trav_prf(&mut self, mut prf: PrfCall<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        for arg in prf.args_mut() {
            Self::Ast::trav_operand(self, arg);
        }
        Expr::PrfCall(prf)
    }

    fn trav_fold(&mut self, mut fold: Fold<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        Self::Ast::trav_operand(self, &mut fold.neutral);

        match &mut fold.foldfun {
            FoldFun::Name(_) => {}
            FoldFun::Apply { args, .. } => {
                for arg in args {
                    if let FoldFunArg::Bound(bound) = arg {
                        Self::Ast::trav_operand(self, bound);
                    }
                }
            }
        }

        fold.selection = match self.trav_tensor(fold.selection) {
            Expr::Tensor(tensor) => tensor,
            _ => unreachable!("trav_tensor must return Tensor"),
        };

        Expr::Fold(fold)
    }

    fn trav_tensor(&mut self, mut tensor: Tensor<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        if let Some(lb) = &mut tensor.lb {
            Self::Ast::trav_operand(self, lb);
        }
        Self::Ast::trav_operand(self, &mut tensor.ub);
        self.trav_body(&mut tensor.body);
        Expr::Tensor(tensor)
    }

    fn trav_array(&mut self, mut array: Array<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        for value in &mut array.elems {
            Self::Ast::trav_operand(self, value);
        }
        Expr::Array(array)
    }

    // Terminals

    fn trav_id(&mut self, _id: &mut Id<'ast, Self::Ast>) {}

    fn trav_const(&mut self, _c: &mut Const) {}

    fn trav_type(&mut self, _ty: &mut Type) {}
}
