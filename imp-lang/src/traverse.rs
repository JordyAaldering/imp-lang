use crate::ast::*;

pub trait Visit<'ast> {
    type Ast: AstConfig + 'ast;

    // Declarations

    fn visit_program(&mut self, program: &Program<'ast, Self::Ast>) {
        for fundef in program.functions.values() {
            self.visit_fundef(fundef);
        }

        for fundef in program.generic_functions.values() {
            self.visit_fargs_poly(&fundef.args);
            for stmt in &fundef.body {
                self.visit_stmt(stmt);
            }
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, Self::Ast>) {
        self.visit_fargs(&fundef.args);

        for vardec in &fundef.decs {
            self.visit_vardec(vardec);
        }

        for stmt in &fundef.body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_fargs(&mut self, args: &[&'ast Farg]) {
        for arg in args {
            self.visit_farg(arg);
        }
    }

    fn visit_farg(&mut self, _arg: &'ast Farg) { }

    fn visit_farg_poly(&mut self, _arg: &PolyArg) { }

    fn visit_fargs_poly(&mut self, args: &[PolyArg]) {
        for arg in args {
            self.visit_farg_poly(arg);
        }
    }

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
            Tensor(n) => self.visit_tensor(n),
            Array(n) => self.visit_array(n),
            Id(n) => self.visit_id(n),
            Bool(n) => self.visit_bool(n),
            U32(n) => self.visit_u32(n),
        }
    }


    fn visit_call(&mut self, call: &Call<'ast, Self::Ast>) {
        for arg in &call.args {
            Self::Ast::visit_operand(self, arg);
        }
    }

    fn visit_prf_call(&mut self, prf_call: &PrfCall<'ast, Self::Ast>) {
        for arg in &prf_call.args {
            Self::Ast::visit_operand(self, arg);
        }
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
        for value in &array.values {
            Self::Ast::visit_operand(self, value);
        }
    }

    // Terminals

    fn visit_id(&mut self, _id: &Id<'ast, Self::Ast>) { }

    fn visit_bool(&mut self, _v: &bool) { }

    fn visit_u32(&mut self, _v: &u32) { }

    fn visit_type(&mut self, _ty: &Type) { }
}

pub trait Rewrite<'ast> {
    type Ast: AstConfig + 'ast;

    // Declarations

    fn rewrite_program(&mut self, program: &mut Program<'ast, Self::Ast>) {
        for fundef in program.functions.values_mut() {
            self.rewrite_fundef(fundef);
        }

        for fundef in program.generic_functions.values_mut() {
            for stmt in &mut fundef.body {
                self.rewrite_stmt(stmt);
            }
        }
    }

    fn rewrite_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        let new_args = fundef.args.iter().map(|&arg| self.rewrite_farg(arg)).collect();
        fundef.args = new_args;
        fundef.ret_type = self.rewrite_type(fundef.ret_type.clone());
        for stmt in &mut fundef.body {
            self.rewrite_stmt(stmt);
        }
    }

    fn rewrite_farg(&mut self, arg: &'ast Farg) -> &'ast Farg {
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
            Tensor(n) => self.rewrite_tensor(n),
            Array(n) => self.rewrite_array(n),
            // Terminals
            Id(n) => Id(self.rewrite_id(n)),
            Bool(v) => Bool(self.rewrite_bool(v)),
            U32(v) => U32(self.rewrite_u32(v)),
        }
    }

    fn rewrite_call(&mut self, call: Call<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        Expr::Call(call)
    }

    fn rewrite_prf_call(&mut self, prf_call: PrfCall<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        Expr::PrfCall(prf_call)
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

    fn rewrite_bool(&mut self, v: bool) -> bool {
        v
    }

    fn rewrite_u32(&mut self, v: u32) -> u32 {
        v
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

        Program {
            functions,
            generic_functions: std::collections::HashMap::new(),
            traits: program.traits,
            impls: program.impls,
        }
    }

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        let args = self.trav_fargs(fundef.args);

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
            decs,
            body,
        }
    }

    fn trav_fargs(&mut self, args: Vec<&'ast Farg>) -> Vec<&'ast Farg> {
        let mut new_args = Vec::new();
        for arg in args {
            new_args.push(self.trav_farg(arg));
        }
        new_args
    }

    fn trav_farg(&mut self, arg: &'ast Farg) -> &'ast Farg {
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

    type CallOut = Call<'ast, Self::OutAst>;

    fn trav_call(&mut self, call: Call<'ast, Self::InAst>) -> Self::CallOut;

    type PrfCallOut = PrfCall<'ast, Self::OutAst>;

    fn trav_prf_call(&mut self, prf_call: PrfCall<'ast, Self::InAst>) -> Self::PrfCallOut;

    type TensorOut = Tensor<'ast, Self::OutAst>;

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut;

    type ArrayOut = Array<'ast, Self::OutAst>;

    fn trav_array(&mut self, array: Array<'ast, Self::InAst>) -> Self::ArrayOut;

    // Terminals

    type IdOut = Id<'ast, Self::OutAst>;

    fn trav_id(&mut self, _id: Id<'ast, Self::InAst>) -> Self::IdOut {
        unimplemented!()
    }

    type BoolOut = bool;

    fn trav_bool(&mut self, _v: bool) -> Self::BoolOut {
        unimplemented!()
    }

    type U32Out = u32;

    fn trav_u32(&mut self, _v: u32) -> Self::U32Out {
        unimplemented!()
    }

    type TypeOut = Type;

    fn trav_type(&mut self, _ty: Type) -> Self::TypeOut {
        unimplemented!()
    }
}
