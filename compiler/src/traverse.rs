use crate::ast::*;

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
        let ret_type = self.trav_ret_type(fundef.ret_type);

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
            args,
            decs,
            body,
            ret_type,
        }
    }

    fn trav_ret_type(&mut self, ty: <Self::InAst as AstConfig>::ValueType) -> <Self::OutAst as AstConfig>::ValueType;

    fn trav_fargs(&mut self, args: Vec<&'ast Avis<Self::InAst>>) -> Vec<&'ast Avis<Self::OutAst>> {
        let mut new_args = Vec::new();
        for arg in args {
            new_args.push(self.trav_farg(arg));
        }
        new_args
    }

    fn trav_farg(&mut self, arg: &'ast Avis<Self::InAst>) -> &'ast Avis<Self::OutAst>;

    fn trav_vardec(&mut self, decl: &'ast Avis<Self::InAst>) -> &'ast Avis<Self::OutAst>;

    ///
    /// Statements
    ///

    fn trav_stmt(&mut self, stmt: Stmt<'ast, Self::InAst>) -> Stmt<'ast, Self::OutAst>;

    type AssignOut = Assign<'ast, Self::OutAst>;

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Self::AssignOut;

    type ReturnOut = Return<'ast, Self::OutAst>;

    fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Self::ReturnOut;

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

    /// An identifier occurring in an expression-position.
    fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Self::IdOut;

    type BoolOut = bool;

    fn trav_bool(&mut self, v: bool) -> Self::BoolOut;

    type U32Out = u32;

    fn trav_u32(&mut self, v: u32) -> Self::U32Out;
}

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

    fn visit_fargs(&mut self, args: &Vec<&'ast Avis<Self::Ast>>) {
        for arg in args {
            self.visit_farg(arg);
        }
    }

    fn visit_farg(&mut self, _arg: &'ast Avis<Self::Ast>) { }

    fn visit_vardec(&mut self, _vardec: &'ast Avis<Self::Ast>) { }

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
        <Self::Ast as AstConfig>::visit_operand(self, &tensor.lb);
        <Self::Ast as AstConfig>::visit_operand(self, &tensor.ub);

        for stmt in &tensor.body {
            self.visit_stmt(stmt);
        }

        <Self::Ast as AstConfig>::visit_operand(self, &tensor.ret);
    }

    fn visit_binary(&mut self, binary: &Binary<'ast, Self::Ast>) {
        <Self::Ast as AstConfig>::visit_operand(self, &binary.l);
        <Self::Ast as AstConfig>::visit_operand(self, &binary.r);
    }

    fn visit_unary(&mut self, unary: &Unary<'ast, Self::Ast>) {
        <Self::Ast as AstConfig>::visit_operand(self, &unary.r);
    }

    ///
    /// Terminals
    ///

    /// An identifier occurring in an expression-position.
    fn visit_id(&mut self, _id: &Id<'ast, Self::Ast>) { }

    fn visit_bool(&mut self, _v: &bool) { }

    fn visit_u32(&mut self, _v: &u32) { }
}

// impl<'ast, T> Traverse<'ast> for T
// where
//     T: Visit<'ast>,
// {
//     type InAst = T::Ast;

//     type OutAst = T::Ast;

//     ///
//     /// Declarations
//     ///

//     fn trav_program(&mut self, program: Program<'ast, Self::InAst>) -> Program<'ast, Self::OutAst> {
//         Visit::visit_program(self, program)
//     }

//     fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
//         Visit::visit_fundef(self, fundef)
//     }

//     fn trav_farg(&mut self, arg: &'ast Avis<Self::InAst>) -> &'ast Avis<Self::OutAst> {
//         Visit::visit_farg(self, arg)
//     }

//     fn trav_vardec(&mut self, vardec: &'ast Avis<Self::InAst>) -> &'ast Avis<Self::OutAst> {
//         Visit::visit_vardec(self, vardec)
//     }

//     ///
//     /// Statements
//     ///

//     fn trav_stmt(&mut self, stmt: Stmt<'ast, Self::InAst>) -> Self::StmtOut {
//         Visit::visit_stmt(self, stmt)
//     }

//     fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Self::AssignOut {
//         Visit::visit_assign(self, assign)
//     }

//     fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Self::ReturnOut {
//         Visit::visit_return(self, ret)
//     }

//     ///
//     /// Expressions
//     ///

//     fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
//         Visit::visit_expr(self, expr)
//     }

//     fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
//         Visit::visit_tensor(self, tensor)
//     }

//     fn trav_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Self::BinaryOut {
//         Visit::visit_binary(self, binary)
//     }

//     fn trav_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Self::UnaryOut {
//         Visit::visit_unary(self, unary)
//     }

//     ///
//     /// Terminals
//     ///

//     fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Self::IdOut {
//         Visit::visit_id(self, id)
//     }

//     fn trav_bool(&mut self, value: bool) -> Self::BoolOut {
//         Visit::visit_bool(self, value)
//     }

//     fn trav_u32(&mut self, value: u32) -> Self::U32Out {
//         Visit::visit_u32(self, value)
//     }
// }
