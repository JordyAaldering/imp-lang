use crate::ast::*;

pub trait Traverse<'ast> {
    type Ast: Invariant + 'ast;

    /// Additional output type for expression traversals, allowing a traversal to produce a value for each expression it visits.
    type ExprOut: Default;

    //
    // Declarations
    //

    fn trav_program(&mut self, program: &mut Program<'ast, Self::Ast>) {
        for (_, fundef) in program.fundefs.iter_mut() {
            self.trav_fundef(fundef);
        }
    }

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        self.trav_fargs(&mut fundef.args);

        for vardec in &fundef.decs {
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

    fn trav_vardec(&mut self, _vardec: &VarInfo<'ast, Self::Ast>) {}

    //
    // Statements
    //

    fn trav_body(&mut self, body: &mut Body<'ast, Self::Ast>) -> Self::ExprOut {
        for stmt in &mut body.stmts {
            self.trav_stmt(stmt);
        }

        Self::Ast::trav_operand(self, &mut body.ret)
    }

    fn trav_stmt(&mut self, stmt: &mut Stmt<'ast, Self::Ast>) {
        match stmt {
            Stmt::Assign(n) => self.trav_assign(n),
            Stmt::Printf(n) => self.trav_printf(n),
        }
    }

    fn trav_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        self.trav_expr(assign.expr);
    }

    fn trav_printf(&mut self, printf: &mut Printf<'ast, Self::Ast>) {
        self.trav_id(&mut printf.id);
    }

    //
    // Expressions
    //

    /// Traverses the expression stored in an [`ExprCell`], allowing the traversal to replace it.
    ///
    /// The current value is swapped out with a placeholder for the duration of the traversal, and the rewritten value is swapped back in afterwards.
    /// Since `cell` may be aliased, every alias observes the rewrite: there is no separate and potentially stale copy.
    fn trav_expr(&mut self, cell: &'ast ExprCell<'ast, Self::Ast>) -> Self::ExprOut {
        let placeholder = Expr::Const(Const::Bool(false));
        let current = cell.replace(placeholder);
        let (rewritten, out) = self.trav_expr_value(current);
        cell.replace(rewritten);
        out
    }

    fn trav_expr_value(&mut self, expr: Expr<'ast, Self::Ast>) -> (Expr<'ast, Self::Ast>, Self::ExprOut) {
        match expr {
            Expr::Cond(n) => self.trav_cond_expr(n),
            Expr::Call(n) => self.trav_call_expr(n),
            Expr::Prf(n) => self.trav_prf_expr(n),
            Expr::Tensor(n) => self.trav_tensor_expr(n),
            Expr::Fold(n) => self.trav_fold_expr(n),
            Expr::Array(n) => self.trav_array_expr(n),
            Expr::Id(n) => self.trav_id_expr(n),
            Expr::Const(n) => self.trav_const_expr(n),
        }
    }

    fn trav_cond_expr(&mut self, mut cond: Cond<'ast, Self::Ast>) -> (Expr<'ast, Self::Ast>, Self::ExprOut) {
        let out = self.trav_cond(&mut cond);
        (Expr::Cond(cond), out)
    }

    fn trav_cond(&mut self, cond: &mut Cond<'ast, Self::Ast>) -> Self::ExprOut {
        Self::Ast::trav_operand(self, &mut cond.cond);

        self.trav_body(&mut cond.then_branch);
        self.trav_body(&mut cond.else_branch);

        Self::ExprOut::default()
    }

    fn trav_call_expr(&mut self, mut call: Call<'ast, Self::Ast>) -> (Expr<'ast, Self::Ast>, Self::ExprOut) {
        let out = self.trav_call(&mut call);
        (Expr::Call(call), out)
    }

    fn trav_call(&mut self, call: &mut Call<'ast, Self::Ast>) -> Self::ExprOut {
        for arg in &mut call.args {
            Self::Ast::trav_operand(self, arg);
        }

        Self::ExprOut::default()
    }

    fn trav_prf_expr(&mut self, mut prf: Prf<'ast, Self::Ast>) -> (Expr<'ast, Self::Ast>, Self::ExprOut) {
        let out = self.trav_prf(&mut prf);
        (Expr::Prf(prf), out)
    }

    fn trav_prf(&mut self, prf: &mut Prf<'ast, Self::Ast>) -> Self::ExprOut {
        for arg in prf.args_mut() {
            Self::Ast::trav_operand(self, arg);
        }

        Self::ExprOut::default()
    }

    fn trav_tensor_expr(&mut self, mut tensor: Tensor<'ast, Self::Ast>) -> (Expr<'ast, Self::Ast>, Self::ExprOut) {
        let out = self.trav_tensor(&mut tensor);
        (Expr::Tensor(tensor), out)
    }

    fn trav_tensor(&mut self, tensor: &mut Tensor<'ast, Self::Ast>) -> Self::ExprOut {
        if let Some(lb) = &mut tensor.lb {
            Self::Ast::trav_operand(self, lb);
        }

        Self::Ast::trav_operand(self, &mut tensor.ub);
        self.trav_body(&mut tensor.body);

        Self::ExprOut::default()
    }

    fn trav_fold_expr(&mut self, mut fold: Fold<'ast, Self::Ast>) -> (Expr<'ast, Self::Ast>, Self::ExprOut) {
        let out = self.trav_fold(&mut fold);
        (Expr::Fold(fold), out)
    }

    fn trav_fold(&mut self, fold: &mut Fold<'ast, Self::Ast>) -> Self::ExprOut {
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

        Self::ExprOut::default()
    }

    fn trav_array_expr(&mut self, mut array: Array<'ast, Self::Ast>) -> (Expr<'ast, Self::Ast>, Self::ExprOut) {
        let out = self.trav_array(&mut array);
        (Expr::Array(array), out)
    }

    fn trav_array(&mut self, array: &mut Array<'ast, Self::Ast>) -> Self::ExprOut {
        for value in &mut array.elems {
            Self::Ast::trav_operand(self, value);
        }

        Self::ExprOut::default()
    }

    fn trav_id_expr(&mut self, mut id: Id<'ast, Self::Ast>) -> (Expr<'ast, Self::Ast>, Self::ExprOut) {
        let out = self.trav_id(&mut id);
        (Expr::Id(id), out)
    }

    fn trav_id(&mut self, _id: &mut Id<'ast, Self::Ast>) -> Self::ExprOut {
        Self::ExprOut::default()
    }

    fn trav_const_expr(&mut self, mut c: Const) -> (Expr<'ast, Self::Ast>, Self::ExprOut) {
        let out = self.trav_const(&mut c);
        (Expr::Const(c), out)
    }

    fn trav_const(&mut self, _c: &mut Const) -> Self::ExprOut {
        Self::ExprOut::default()
    }

    fn trav_type(&mut self, _ty: &Type) -> Self::ExprOut {
        Self::ExprOut::default()
    }
}
