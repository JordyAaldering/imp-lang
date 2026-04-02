use crate::ast::*;

pub trait Visit<'ast> {
    type Ast: AstConfig + 'ast;

    ///
    /// Declarations
    ///

    fn visit_program(&mut self, program: &Program<'ast, Self::Ast>) {
        for fundef in &program.fundefs {
            self.visit_fundef(fundef);
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

    fn visit_fargs(&mut self, args: &Vec<&'ast Farg>) {
        for arg in args {
            self.visit_farg(arg);
        }
    }

    fn visit_farg(&mut self, _arg: &'ast Farg) { }

    fn visit_vardec(&mut self, _vardec: &'ast VarInfo<'ast, Self::Ast>) { }

    ///
    /// Statements
    ///

    fn visit_stmt(&mut self, stmt: &Stmt<'ast, Self::Ast>) {
        match stmt {
            Stmt::Assign(assign) => self.visit_assign(assign),
            Stmt::Return(ret) => self.visit_return(ret),
        }
    }

    fn visit_assign(&mut self, _assign: &Assign<'ast, Self::Ast>) { }

    fn visit_return(&mut self, _ret: &Return<'ast, Self::Ast>) { }

    ///
    /// Expressions
    ///

    fn visit_expr(&mut self, expr: &Expr<'ast, Self::Ast>) {
        use Expr::*;
        match expr {
            Tensor(n) => self.visit_tensor(n),
            Binary(n) => self.visit_binary(n),
            Unary(n) => self.visit_unary(n),
            Id(n) => self.visit_id(n),
            Bool(n) => self.visit_bool(n),
            U32(n) => self.visit_u32(n),
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

    fn visit_binary(&mut self, binary: &Binary<'ast, Self::Ast>) {
        Self::Ast::visit_operand(self, &binary.l);
        Self::Ast::visit_operand(self, &binary.r);
    }

    fn visit_unary(&mut self, unary: &Unary<'ast, Self::Ast>) {
        Self::Ast::visit_operand(self, &unary.r);
    }

    ///
    /// Terminals
    ///

    fn visit_id(&mut self, _id: &Id<'ast, Self::Ast>) { }

    fn visit_bool(&mut self, _v: &bool) { }

    fn visit_u32(&mut self, _v: &u32) { }

    fn visit_type(&mut self, _ty: &Type) { }
}

pub trait Rewrite<'ast> {
    type Ast: AstConfig + 'ast;

    ///
    /// Declarations
    ///

    fn rewrite_program(&mut self, program: &mut Program<'ast, Self::Ast>) {
        for fundef in &mut program.fundefs {
            self.rewrite_fundef(fundef);
        }
    }

    fn rewrite_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        for stmt in &mut fundef.body {
            self.rewrite_stmt(stmt);
        }
    }

    ///
    /// Statements
    ///

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

    ///
    /// Expressions
    ///

    fn rewrite_expr(&mut self, expr: Expr<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        use Expr::*;
        match expr {
            Tensor(n) => Tensor(self.rewrite_tensor(n)),
            Binary(n) => self.rewrite_binary(n),
            Unary(n) => self.rewrite_unary(n),
            Id(n) => Id(self.rewrite_id(n)),
            Bool(v) => Bool(self.rewrite_bool(v)),
            U32(v) => U32(self.rewrite_u32(v)),
        }
    }

    fn rewrite_tensor(&mut self, tensor: Tensor<'ast, Self::Ast>) -> Tensor<'ast, Self::Ast> {
        let mut tensor = tensor;
        for stmt in &mut tensor.body {
            self.rewrite_stmt(stmt);
        }
        tensor
    }

    fn rewrite_binary(&mut self, binary: Binary<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        Expr::Binary(binary)
    }

    fn rewrite_unary(&mut self, unary: Unary<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        Expr::Unary(unary)
    }

    ///
    /// Terminals
    ///

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

    ///
    /// Declarations
    ///

    fn trav_program(&mut self, program: Program<'ast, Self::InAst>) -> Program<'ast, Self::OutAst> {
        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            fundefs.push(self.trav_fundef(fundef));
        }

        Program { fundefs }
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

    ///
    /// Statements
    ///

    fn trav_stmt(&mut self, stmt: Stmt<'ast, Self::InAst>) -> Stmt<'ast, Self::OutAst> {
        use Stmt::*;
        match stmt {
            Assign(n) => Assign(self.trav_assign(n)),
            Return(n) => Return(self.trav_return(n)),
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst>;

    fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Return<'ast, Self::OutAst>;

    ///
    /// Expressions
    ///

    type ExprOut = Expr<'ast, Self::OutAst>;

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut;

    type TensorOut = Tensor<'ast, Self::OutAst>;

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut;

    type BinaryOut = Binary<'ast, Self::OutAst>;

    fn trav_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Self::BinaryOut;

    type UnaryOut = Unary<'ast, Self::OutAst>;

    fn trav_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Self::UnaryOut;

    ///
    /// Terminals
    ///

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
