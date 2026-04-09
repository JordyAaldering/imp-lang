use crate::ast::*;

pub trait Visit<'ast> {
    type Ast: AstConfig + 'ast;

    // Declarations

    fn visit_program(&mut self, program: &Program<'ast, Self::Ast>) {
        for fundef in program.functions.values() {
            self.visit_fundef(fundef);
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, Self::Ast>) {
        self.visit_fargs(&fundef.args);

        for assign in &fundef.shape_prelude {
            self.visit_assign(assign);
        }

        for vardec in &fundef.decs {
            self.visit_vardec(vardec);
        }

        for stmt in &fundef.body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_fargs(&mut self, args: &[Farg]) {
        for arg in args {
            self.visit_farg(arg);
        }
    }

    fn visit_farg(&mut self, _arg: &Farg) { }

    fn visit_vardec(&mut self, _vardec: &'ast VarInfo<'ast, Self::Ast>) { }

    // Statements

    fn visit_stmt(&mut self, stmt: &Stmt<'ast, Self::Ast>) {
        match stmt {
            Stmt::Assign(assign) => self.visit_assign(assign),
            Stmt::Return(ret) => self.visit_return(ret),
        }
    }

    fn visit_assign(&mut self, _assign: &Assign<'ast, Self::Ast>) { }

    fn visit_return(&mut self, _ret: &Return<'ast, Self::Ast>) { }

    // Expressions

    fn visit_expr(&mut self, expr: &Expr<'ast, Self::Ast>) {
        use Expr::*;
        match expr {
            Call(n) => self.visit_call(n),
            PrfCall(n) => self.visit_prf_call(n),
            Fold(n) => self.visit_fold(n),
            Tensor(n) => self.visit_tensor(n),
            Array(n) => self.visit_array(n),
            Id(n) => self.visit_id(n),
            Const(n) => self.visit_const(n),
        }
    }

    fn visit_cond(&mut self, cond: &Cond<'ast, Self::Ast>) {
        Self::Ast::visit_operand(self, &cond.cond);
        Self::Ast::visit_operand(self, &cond.true_branch);
        Self::Ast::visit_operand(self, &cond.false_branch);
    }

    fn visit_call(&mut self, call: &Call<'ast, Self::Ast>) {
        for arg in &call.args {
            Self::Ast::visit_operand(self, arg);
        }
    }

    fn visit_prf_call(&mut self, prf_call: &PrfCall<'ast, Self::Ast>) {
        for arg in prf_call.args() {
            Self::Ast::visit_operand(self, arg);
        }
    }

    fn visit_fold(&mut self, fold: &Fold<'ast, Self::Ast>) {
        Self::Ast::visit_operand(self, &fold.neutral);

        match &fold.foldfun {
            FoldFun::Name(_) => {}
            FoldFun::Apply { args, .. } => {
                for arg in args {
                    if let FoldFunArg::Bound(bound) = arg {
                        Self::Ast::visit_operand(self, bound);
                    }
                }
            }
        }

        self.visit_tensor(&fold.selection);
    }

    fn visit_tensor(&mut self, tensor: &Tensor<'ast, Self::Ast>) {
        Self::Ast::visit_operand(self, &tensor.lb);
        Self::Ast::visit_operand(self, &tensor.ub);

        for stmt in &tensor.body {
            self.visit_stmt(stmt);
        }

        Self::Ast::visit_operand(self, &tensor.ret);
    }

    fn visit_array(&mut self, array: &Array<'ast, Self::Ast>) {
        for value in &array.elems {
            Self::Ast::visit_operand(self, value);
        }
    }

    // Terminals

    fn visit_id(&mut self, _id: &Id<'ast, Self::Ast>) { }

    fn visit_const(&mut self, _c: &Const) { }

    fn visit_type(&mut self, _ty: &Type) { }
}

pub trait Rewrite<'ast> {
    type Ast: AstConfig + 'ast;

    // Declarations

    fn rewrite_program(&mut self, program: &mut Program<'ast, Self::Ast>) {
        for fundef in program.functions.values_mut() {
            self.rewrite_fundef(fundef);
        }
    }

    fn rewrite_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        for arg in &mut fundef.args {
            *arg = self.rewrite_farg(arg.clone());
        }

        fundef.ret_type = self.rewrite_type(fundef.ret_type.clone());

        for assign in &mut fundef.shape_prelude {
            self.rewrite_assign(assign);
        }

        for stmt in &mut fundef.body {
            self.rewrite_stmt(stmt);
        }
    }

    fn rewrite_farg(&mut self, arg: Farg) -> Farg {
        arg
    }

    // Statements

    fn rewrite_stmt(&mut self, stmt: &mut Stmt<'ast, Self::Ast>) {
        match stmt {
            Stmt::Assign(assign) => self.rewrite_assign(assign),
            Stmt::Return(ret) => self.rewrite_return(ret),
        }
    }

    fn rewrite_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        let new_expr = self.rewrite_expr((*assign.expr).clone());
        assign.expr = Box::leak(Box::new(new_expr));
    }

    fn rewrite_return(&mut self, ret: &mut Return<'ast, Self::Ast>) {
        ret.id = self.rewrite_id(ret.id.clone());
    }

    // Expressions

    fn rewrite_expr(&mut self, expr: Expr<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        use Expr::*;
        match expr {
            Call(n) => self.rewrite_call(n),
            PrfCall(n) => self.rewrite_prf_call(n),
            Fold(n) => self.rewrite_fold(n),
            Tensor(n) => self.rewrite_tensor(n),
            Array(n) => self.rewrite_array(n),
            // Terminals
            Id(n) => Id(self.rewrite_id(n)),
            Const(n) => Const(self.rewrite_const(n)),
        }
    }

    fn rewrite_cond(&mut self, cond: Cond<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        todo!()
    }

    fn rewrite_call(&mut self, call: Call<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        Expr::Call(call)
    }

    fn rewrite_prf_call(&mut self, prf_call: PrfCall<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        Expr::PrfCall(prf_call)
    }

    fn rewrite_fold(&mut self, fold: Fold<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        Expr::Fold(fold)
    }

    fn rewrite_tensor(&mut self, tensor: Tensor<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        let mut tensor = tensor;
        for stmt in &mut tensor.body {
            self.rewrite_stmt(stmt);
        }
        Expr::Tensor(tensor)
    }

    fn rewrite_array(&mut self, array: Array<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        Expr::Array(array)
    }

    // Terminals

    fn rewrite_id(&mut self, id: Id<'ast, Self::Ast>) -> Id<'ast, Self::Ast> {
        id
    }

    fn rewrite_const(&mut self, c: Const) -> Const {
        c
    }

    fn rewrite_type(&mut self, ty: Type) -> Type {
        ty
    }
}

pub trait Traverse<'ast> {
    type InAst: AstConfig;

    type OutAst: AstConfig + 'ast;

    // Declarations

    fn trav_program(&mut self, program: Program<'ast, Self::InAst>) -> Program<'ast, Self::OutAst> {
        let mut functions = std::collections::HashMap::new();
        for (name, fundef) in program.functions {
            functions.insert(name, self.trav_fundef(fundef));
        }

        Program { functions }
    }

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        let args = self.trav_fargs(fundef.args);

        let mut shape_prelude = Vec::new();
        for assign in fundef.shape_prelude {
            shape_prelude.push(self.trav_assign(assign));
        }

        let mut decs = Vec::new();
        for vardec in fundef.decs {
            decs.push(self.trav_vardec(vardec));
        }

        let mut body = Vec::new();
        for stmt in fundef.body {
            body.push(self.trav_stmt(stmt));
        }

        Fundef {
            name: fundef.name,
            ret_type: fundef.ret_type,
            args,
            shape_prelude,
            shape_facts: ShapeFacts::default(),
            decs,
            body,
        }
    }

    fn trav_fargs(&mut self, args: Vec<Farg>) -> Vec<Farg> {
        let mut new_args = Vec::new();
        for arg in args {
            new_args.push(self.trav_farg(arg));
        }
        new_args
    }

    fn trav_farg(&mut self, arg: Farg) -> Farg {
        arg
    }

    fn trav_vardec(&mut self, _decl: &'ast VarInfo<'ast, Self::InAst>) -> &'ast VarInfo<'ast, Self::OutAst> {
        unimplemented!()
    }

    // Statements

    fn trav_stmt(&mut self, stmt: Stmt<'ast, Self::InAst>) -> Stmt<'ast, Self::OutAst> {
        use Stmt::*;
        match stmt {
            Assign(n) => Assign(self.trav_assign(n)),
            Return(n) => Return(self.trav_return(n)),
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst>;

    fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Return<'ast, Self::OutAst>;

    // Expressions

    type ExprOut = Expr<'ast, Self::OutAst>;

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut;

    // type CondOut = Cond<'ast, Self::OutAst>;

    // fn trav_cond(&mut self, cond: Cond<'ast, Self::InAst>) -> Self::CondOut;

    type CallOut = Call<'ast, Self::OutAst>;

    fn trav_call(&mut self, call: Call<'ast, Self::InAst>) -> Self::CallOut;

    type PrfCallOut = PrfCall<'ast, Self::OutAst>;

    fn trav_prf_call(&mut self, prf_call: PrfCall<'ast, Self::InAst>) -> Self::PrfCallOut;

    type FoldOut = Fold<'ast, Self::OutAst>;

    fn trav_fold(&mut self, fold: Fold<'ast, Self::InAst>) -> Self::FoldOut;

    type TensorOut = Tensor<'ast, Self::OutAst>;

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut;

    type ArrayOut = Array<'ast, Self::OutAst>;

    fn trav_array(&mut self, array: Array<'ast, Self::InAst>) -> Self::ArrayOut;

    // Terminals

    type IdOut = Id<'ast, Self::OutAst>;

    fn trav_id(&mut self, _id: Id<'ast, Self::InAst>) -> Self::IdOut {
        unimplemented!()
    }

    type ConstOut = Const;

    fn trav_const(&mut self, _c: Const) -> Self::ConstOut {
        unimplemented!()
    }

    type TypeOut = Type;

    fn trav_type(&mut self, _ty: Type) -> Self::TypeOut {
        unimplemented!()
    }
}
