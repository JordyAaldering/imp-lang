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
            let fundef = self.trav_fundef(fundef);
            fundefs.push(fundef);
        }

        Program { fundefs }
    }

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst>;

    fn trav_farg(&mut self, arg: &'ast Avis<Self::InAst>) -> &'ast Avis<Self::OutAst>;

    fn trav_vardec(&mut self, decl: &'ast Avis<Self::InAst>) -> &'ast Avis<Self::OutAst>;

    ///
    /// Statements
    ///

    type StmtOut = Stmt<'ast, Self::OutAst>;

    fn trav_stmt(&mut self, stmt: Stmt<'ast, Self::InAst>) -> Self::StmtOut;

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

    fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Self::IdOut;

    type BoolOut = bool;

    fn trav_bool(&mut self, value: bool) -> Self::BoolOut;

    type U32Out = u32;

    fn trav_u32(&mut self, value: u32) -> Self::U32Out;
}

pub trait Visit<'ast> {
    type Ast: AstConfig + 'ast;

    ///
    /// Declarations
    ///

    fn visit_program(&mut self, program: Program<'ast, Self::Ast>) -> Program<'ast, Self::Ast> {
        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            fundefs.push(self.visit_fundef(fundef));
        }
        Program { fundefs }
    }

    fn visit_fundef(&mut self, fundef: Fundef<'ast, Self::Ast>) -> Fundef<'ast, Self::Ast> {
        let mut args = Vec::with_capacity(fundef.args.len());
        for arg in fundef.args {
            args.push(self.visit_farg(arg));
        }

        let mut decls = Vec::with_capacity(fundef.decs.len());
        for vardec in fundef.decs {
            decls.push(self.visit_vardec(vardec));
        }

        let mut body = Vec::new();
        for stmt in fundef.body {
            body.push(self.visit_stmt(stmt));
        }

        Fundef { name: fundef.name, args, decs: decls, body }
    }

    fn visit_farg(&mut self, arg: &'ast Avis<Self::Ast>) -> &'ast Avis<Self::Ast> {
        arg
    }

    fn visit_vardec(&mut self, vardec: &'ast Avis<Self::Ast>) -> &'ast Avis<Self::Ast> {
        vardec
    }

    ///
    /// Statements
    ///

    fn visit_stmt(&mut self, stmt: Stmt<'ast, Self::Ast>) -> Stmt<'ast, Self::Ast> {
        match stmt {
            Stmt::Assign(assign) => Stmt::Assign(self.visit_assign(assign)),
            Stmt::Return(ret) => Stmt::Return(self.visit_return(ret)),
        }
    }

    fn visit_assign(&mut self, assign: Assign<'ast, Self::Ast>) -> Assign<'ast, Self::Ast> {
        assign
    }

    fn visit_return(&mut self, ret: Return<'ast, Self::Ast>) -> Return<'ast, Self::Ast> {
        ret
    }

    ///
    /// Expressions
    ///

    fn visit_expr(&mut self, expr: Expr<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        expr
    }

    fn visit_tensor(&mut self, tensor: Tensor<'ast, Self::Ast>) -> Tensor<'ast, Self::Ast> {
        let body = tensor
            .body
            .into_iter()
            .map(|stmt| self.visit_stmt(stmt))
            .collect();
        let lb = self.visit_id(tensor.lb);
        let ub = self.visit_id(tensor.ub);
        let ret = self.visit_id(tensor.ret);
        Tensor {
            body,
            iv: tensor.iv,
            lb,
            ub,
            ret,
        }
    }

    fn visit_binary(&mut self, binary: Binary<'ast, Self::Ast>) -> Binary<'ast, Self::Ast> {
        binary
    }

    fn visit_unary(&mut self, unary: Unary<'ast, Self::Ast>) -> Unary<'ast, Self::Ast> {
        unary
    }

    ///
    /// Terminals
    ///

    fn visit_id(&mut self, id: Id<'ast, Self::Ast>) -> Id<'ast, Self::Ast> {
        id
    }

    fn visit_bool(&mut self, value: bool) -> bool {
        value
    }

    fn visit_u32(&mut self, value: u32) -> u32 {
        value
    }
}

impl<'ast, T> Traverse<'ast> for T
where
    T: Visit<'ast>,
{
    type InAst = T::Ast;

    type OutAst = T::Ast;

    ///
    /// Declarations
    ///

    fn trav_program(&mut self, program: Program<'ast, Self::InAst>) -> Program<'ast, Self::OutAst> {
        Visit::visit_program(self, program)
    }

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        Visit::visit_fundef(self, fundef)
    }

    fn trav_farg(&mut self, arg: &'ast Avis<Self::InAst>) -> &'ast Avis<Self::OutAst> {
        Visit::visit_farg(self, arg)
    }

    fn trav_vardec(&mut self, vardec: &'ast Avis<Self::InAst>) -> &'ast Avis<Self::OutAst> {
        Visit::visit_vardec(self, vardec)
    }

    ///
    /// Statements
    ///

    fn trav_stmt(&mut self, stmt: Stmt<'ast, Self::InAst>) -> Self::StmtOut {
        Visit::visit_stmt(self, stmt)
    }

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Self::AssignOut {
        Visit::visit_assign(self, assign)
    }

    fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Self::ReturnOut {
        Visit::visit_return(self, ret)
    }

    ///
    /// Expressions
    ///

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
        Visit::visit_expr(self, expr)
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
        Visit::visit_tensor(self, tensor)
    }

    fn trav_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Self::BinaryOut {
        Visit::visit_binary(self, binary)
    }

    fn trav_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Self::UnaryOut {
        Visit::visit_unary(self, unary)
    }

    ///
    /// Terminals
    ///

    fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Self::IdOut {
        Visit::visit_id(self, id)
    }

    fn trav_bool(&mut self, value: bool) -> Self::BoolOut {
        Visit::visit_bool(self, value)
    }

    fn trav_u32(&mut self, value: u32) -> Self::U32Out {
        Visit::visit_u32(self, value)
    }
}
