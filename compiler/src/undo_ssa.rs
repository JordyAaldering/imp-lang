use crate::{ast::{self, ArgOrVar, Avis, TypedAst}, compile::compile_ast::*};

pub struct UndoSsa<'ast> {
    args: Vec<&'ast Avis<'ast, TypedAst>>,
    scopes: Vec<Vec<(&'ast ast::Avis<'ast, TypedAst>, &'ast ast::Expr<'ast, TypedAst>)>>,
}

impl<'ast> UndoSsa<'ast> {
    pub fn new() -> Self {
        Self { args: Vec::new(), scopes: Vec::new() }
    }

    fn find(&self, key: ArgOrVar<'ast, TypedAst>) -> &'ast Avis<'ast, TypedAst> {
        match key {
            ArgOrVar::Arg(i) => self.args[i],
            ArgOrVar::Var(v) | ArgOrVar::Iv(v) => v,
        }
    }

    fn find_ssa(&self, key: &'ast ast::Avis<'ast, TypedAst>) -> &'ast ast::Expr<'ast, TypedAst> {
        for scope in self.scopes.iter().rev() {
            for (id, expr) in scope.iter().rev() {
                if std::ptr::eq(*id, key) {
                    return *expr;
                }
            }
        }
        unreachable!()
    }

    pub fn trav_program(&mut self, program: &ast::Program<'ast, TypedAst>) -> Program {
        let fundefs = program.fundefs.iter().map(|f| self.trav_fundef(f)).collect();
        Program { fundefs }
    }

    fn trav_fundef(&mut self, fundef: &ast::Fundef<'ast, TypedAst>) -> Fundef {
        self.args = fundef.args.clone();
        self.scopes.push(fundef.ssa.clone());

        let args = fundef.args.iter().map(|a| (a.ty.clone(), a.name.clone())).collect();

        let mut body = Vec::new();
        body.push(self.generate_assignment(fundef.ret, fundef));
        body.push(Stmt::Return { expr: Expr::Identifier(self.find(fundef.ret).name.to_owned()) });

        self.scopes.pop().unwrap();
        Fundef {
            name: fundef.name.to_owned(),
            ret_type: self.find(fundef.ret).ty.to_owned(),
            args,
            block: Block { stmts: body },
        }
    }

    fn generate_assignment(&mut self, id: ArgOrVar<'ast, TypedAst>, fundef: &ast::Fundef<'ast, TypedAst>) -> Stmt {
        let lhs = self.find(id).name.clone();
        let expr = self.inline_expr(id, fundef);
        Stmt::Assign { lhs, expr }
    }

    fn inline_expr(&mut self, id: ArgOrVar<'ast, TypedAst>, fundef: &ast::Fundef<'ast, TypedAst>) -> Expr {
        match id {
            ArgOrVar::Arg(i) => Expr::Identifier(fundef.args[i].name.clone()),
            ArgOrVar::Var(k) => {
                match self.find_ssa(k).clone() {
                    ast::Expr::Tensor(ast::Tensor { iv, lb, ub, ret, ssa }) => {
                        self.scopes.push(ssa.clone());
                        let iv = iv.name.clone();
                        let expr = self.inline_expr(ret, fundef);
                        let lb = self.inline_expr(lb, fundef);
                        let ub = self.inline_expr(ub, fundef);
                        self.scopes.pop().unwrap();
                        Expr::Tensor { iv, expr: Box::new(expr), lb: Box::new(lb), ub: Box::new(ub) }
                    }
                    ast::Expr::Binary(ast::Binary { l, r, op }) => {
                        let l = self.inline_expr(l, fundef);
                        let r = self.inline_expr(r, fundef);
                        Expr::Binary { l: Box::new(l), r: Box::new(r), op }
                    }
                    ast::Expr::Unary(ast::Unary { r, op }) => {
                        let r = self.inline_expr(r, fundef);
                        Expr::Unary { r: Box::new(r), op }
                    }
                    ast::Expr::Bool(v) => Expr::Bool(v),
                    ast::Expr::U32(v) => Expr::U32(v),
                }
            }
            ArgOrVar::Iv(k) => Expr::Identifier(k.name.clone()),
        }
    }
}
