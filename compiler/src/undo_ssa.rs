use crate::{ast::{self, ArgOrVar, Avis, TypedAst}, compile::compile_ast::*};

/// Convert SSA IR back to simple sequential IR.
///
/// Transforms ast::Program (SSA form with scoped bindings) to
/// compile_ast::Program (simple sequential statements with string identifiers).
/// This reverses the SSA transformation, inlining variable definitions to
/// create a flat, statement-based program suitable for undo_ssa.
///
/// Note: This is a cross-AST conversion (ast::* -> compile_ast::*) and thus
/// uses manual traversal rather than AstPass (which is designed for same-AST transforms).
pub struct UndoSsa<'ast> {
    args: Vec<&'ast Avis<TypedAst>>,
    scopes: Vec<ast::ScopeBlock<'ast, TypedAst>>,
}

impl<'ast> UndoSsa<'ast> {
    pub fn new() -> Self {
        Self { args: Vec::new(), scopes: Vec::new() }
    }

    fn find(&self, key: ArgOrVar<'ast, TypedAst>) -> &'ast Avis<TypedAst> {
        match key {
            ArgOrVar::Arg(i) => self.args[i],
            ArgOrVar::Var(v) => v,
        }
    }

    fn find_local_def(&self, key: &'ast ast::Avis<TypedAst>) -> ast::LocalDef<'ast, TypedAst> {
        ast::find_local_in_scopes(&self.scopes, key).expect("missing local definition in undo_ssa")
    }

    pub fn trav_program(&mut self, program: &ast::Program<'ast, TypedAst>) -> Program {
        let fundefs = program.fundefs.iter().map(|f| self.trav_fundef(f)).collect();
        Program { fundefs }
    }

    fn trav_fundef(&mut self, fundef: &ast::Fundef<'ast, TypedAst>) -> Fundef {
        self.args = fundef.args.clone();
        let scope = fundef
            .body
            .iter()
            .filter_map(|stmt| (*stmt).as_scope_entry())
            .collect::<ast::ScopeBlock<'ast, TypedAst>>();
        self.scopes.push(scope);

        let args = fundef.args.iter().map(|a| (a.ty.clone(), a.name.clone())).collect();

        let ret = fundef.ret_id();
        let mut body = Vec::new();
        body.push(self.generate_assignment(ret, fundef));
        body.push(Stmt::Return { expr: Expr::Identifier(self.find(ret).name.to_owned()) });

        self.scopes.pop().unwrap();
        Fundef {
            name: fundef.name.to_owned(),
            ret_type: self.find(ret).ty.to_owned(),
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
                match self.find_local_def(k) {
                    ast::LocalDef::Assign(expr) => {
                        match expr.clone() {
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
                    ast::LocalDef::IndexRange { .. } => Expr::Identifier(k.name.clone()),
                }
            }
        }
    }
}
