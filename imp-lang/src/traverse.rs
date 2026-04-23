use std::ptr;

use crate::ast::*;

pub trait Traverse<'ast> {
    type Ast: AstConfig + 'ast;

    // Declarations

    fn trav_program(&mut self, program: &mut Program<'ast, Self::Ast>) {
        for fundef in program.fundefs.iter_mut() {
            self.trav_fundef(fundef);
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

    fn trav_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        self.trav_expr(assign.expr);
    }

    fn trav_printf(&mut self, printf: &mut Printf<'ast, Self::Ast>) {
        self.trav_id(&mut printf.id);
    }

    // Expressions

    fn trav_expr(&mut self, expr: &'ast Expr<'ast, Self::Ast>) {
        self.trav_expr_ptr(expr as *const Expr<'ast, Self::Ast> as *mut Expr<'ast, Self::Ast>);
    }

    fn trav_expr_ptr(&mut self, expr: *mut Expr<'ast, Self::Ast>) {
        let current = unsafe { ptr::read(expr) };
        let rewritten = self.trav_expr_value(current);
        unsafe { ptr::write(expr, rewritten); }
    }

    fn trav_expr_value(&mut self, expr: Expr<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        use Expr::*;
        match expr {
            Cond(n) => self.trav_cond_expr(n),
            Call(n) => self.trav_call_expr(n),
            PrfCall(n) => self.trav_prf_expr(n),
            Tensor(n) => self.trav_tensor_expr(n),
            Fold(n) => self.trav_fold_expr(n),
            Array(n) => self.trav_array_expr(n),
            Id(n) => self.trav_id_expr(n),
            Const(n) => self.trav_const_expr(n),
        }
    }

    fn trav_cond_expr(&mut self, mut cond: Cond<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        self.trav_cond(&mut cond);
        Expr::Cond(cond)
    }

    fn trav_cond(&mut self, cond: &mut Cond<'ast, Self::Ast>) {
        Self::Ast::trav_operand(self, &mut cond.cond);
        self.trav_body(&mut cond.then_branch);
        self.trav_body(&mut cond.else_branch);
    }

    fn trav_call_expr(&mut self, mut call: Call<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        self.trav_call(&mut call);
        Expr::Call(call)
    }

    fn trav_call(&mut self, call: &mut Call<'ast, Self::Ast>) {
        for arg in &mut call.args {
            Self::Ast::trav_operand(self, arg);
        }
    }

    fn trav_prf_expr(&mut self, mut prf: PrfCall<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        self.trav_prf(&mut prf);
        Expr::PrfCall(prf)
    }

    fn trav_prf(&mut self, prf: &mut PrfCall<'ast, Self::Ast>) {
        for arg in prf.args_mut() {
            Self::Ast::trav_operand(self, arg);
        }
    }

    fn trav_fold_expr(&mut self, mut fold: Fold<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        self.trav_fold(&mut fold);
        Expr::Fold(fold)
    }

    fn trav_fold(&mut self, fold: &mut Fold<'ast, Self::Ast>) {
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

        self.trav_tensor(&mut fold.selection);
    }

    fn trav_tensor_expr(&mut self, mut tensor: Tensor<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        self.trav_tensor(&mut tensor);
        Expr::Tensor(tensor)
    }

    fn trav_tensor(&mut self, tensor: &mut Tensor<'ast, Self::Ast>) {
        if let Some(lb) = &mut tensor.lb {
            Self::Ast::trav_operand(self, lb);
        }
        Self::Ast::trav_operand(self, &mut tensor.ub);
        self.trav_body(&mut tensor.body);
    }

    fn trav_array_expr(&mut self, mut array: Array<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        self.trav_array(&mut array);
        Expr::Array(array)
    }

    fn trav_array(&mut self, array: &mut Array<'ast, Self::Ast>) {
        for value in &mut array.elems {
            Self::Ast::trav_operand(self, value);
        }
    }

    fn trav_id_expr(&mut self, mut id: Id<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        self.trav_id(&mut id);
        Expr::Id(id)
    }

    fn trav_id(&mut self, _id: &mut Id<'ast, Self::Ast>) {}

    fn trav_const_expr(&mut self, mut c: Const) -> Expr<'ast, Self::Ast> {
        self.trav_const(&mut c);
        Expr::Const(c)
    }

    fn trav_const(&mut self, _c: &mut Const) {}

    fn trav_type(&mut self, _ty: &mut Type) {}
}
