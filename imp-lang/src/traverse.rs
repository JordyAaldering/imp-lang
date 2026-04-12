use std::collections::HashMap;

use crate::ast::*;

pub trait Visit<'ast> {
    type Ast: AstConfig + 'ast;

    // Declarations

    fn visit_program(&mut self, program: &Program<'ast, Self::Ast>) {
        for (_, groups) in &program.overloads {
            for (_, fundefs) in groups {
                for fundef in fundefs {
                    self.visit_fundef(&fundef);
                }
            }
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

        self.visit_body(&fundef.body);
    }

    fn visit_fargs(&mut self, args: &[Farg]) {
        for arg in args {
            self.visit_farg(arg);
        }
    }

    fn visit_farg(&mut self, _arg: &Farg) { }

    fn visit_vardec(&mut self, _vardec: &'ast VarInfo<'ast, Self::Ast>) { }

    // Statements

    fn visit_body(&mut self, body: &Body<'ast, Self::Ast>) {
        for stmt in &body.stmts {
            self.visit_stmt(stmt);
        }
        Self::Ast::visit_operand(self, &body.ret);
    }

    fn visit_stmt(&mut self, stmt: &Stmt<'ast, Self::Ast>) {
        match stmt {
            Stmt::Assign(assign) => self.visit_assign(assign),
        }
    }

    fn visit_assign(&mut self, _assign: &Assign<'ast, Self::Ast>) { }

    // Expressions

    fn visit_expr(&mut self, expr: &Expr<'ast, Self::Ast>) {
        use Expr::*;
        match expr {
            Cond(n) => self.visit_cond(n),
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
        Self::Ast::visit_operand(self, &cond.then_branch);
        Self::Ast::visit_operand(self, &cond.else_branch);
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
        if let Some(lb) = &tensor.lb {
            Self::Ast::visit_operand(self, lb);
        }
        Self::Ast::visit_operand(self, &tensor.ub);
        self.visit_body(&tensor.body);
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
        for groups in program.overloads.values_mut() {
            for fundefs in groups.values_mut() {
                for fundef in fundefs {
                    self.rewrite_fundef(fundef);
                }
            }
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

        self.rewrite_body(&mut fundef.body);
    }

    fn rewrite_farg(&mut self, arg: Farg) -> Farg {
        arg
    }

    // Statements

    fn rewrite_body(&mut self, body: &mut Body<'ast, Self::Ast>);

    fn rewrite_stmt(&mut self, stmt: &mut Stmt<'ast, Self::Ast>) {
        match stmt {
            Stmt::Assign(assign) => self.rewrite_assign(assign),
        }
    }

    fn rewrite_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        let new_expr = self.rewrite_expr((*assign.expr).clone());
        assign.expr = Box::leak(Box::new(new_expr));
    }

    // Expressions

    fn rewrite_expr(&mut self, expr: Expr<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        use Expr::*;
        match expr {
            Cond(n) => self.rewrite_cond(n),
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
        Expr::Cond(cond)
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
        self.rewrite_body(&mut tensor.body);
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
        let mut overloads = HashMap::new();

        for (name, groups) in program.overloads {
            let mut new_groups = HashMap::new();

            for (sig, fundefs) in groups {
                let mut new_fundefs = Vec::new();

                for fundef in fundefs {
                    new_fundefs.push(self.trav_fundef(fundef));
                }

                new_groups.insert(sig, new_fundefs);
            }

            overloads.insert(name, new_groups);
        }

        Program { overloads }
    }

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst>;

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

    type BodyOut = Body<'ast, Self::OutAst>;

    fn trav_body(&mut self, body: Body<'ast, Self::InAst>) -> Self::BodyOut;

    fn trav_stmt(&mut self, stmt: Stmt<'ast, Self::InAst>) -> Stmt<'ast, Self::OutAst> {
        use Stmt::*;
        match stmt {
            Assign(n) => Assign(self.trav_assign(n)),
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst>;

    // Expressions

    type ExprOut = Expr<'ast, Self::OutAst>;

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut;

    type CondOut = Cond<'ast, Self::OutAst>;

    fn trav_cond(&mut self, cond: Cond<'ast, Self::InAst>) -> Self::CondOut;

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
}
