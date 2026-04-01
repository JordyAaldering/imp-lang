use std::collections::HashMap;

use crate::{ast::*, scanparse::parse_ast};

/// Parse AST to SSA IR conversion.
///
/// Transforms the parse_ast::Program (simple variable tracking) into an SSA-like
/// ast::Program where each variable reference is traced to its SSA assignment, and
/// fresh UIDs are created for each intermediate value.
pub fn convert_to_ssa<'ast>(program: parse_ast::Program) -> Program<'ast, UntypedAst> {
    let fundefs = program.fundefs.into_iter()
        .map(|f| ConvertToSsa::new().convert_fundef(f))
        .collect();
    Program { fundefs }
}

pub struct ConvertToSsa<'ast> {
    uid: usize,
    ids: Vec<&'ast Avis<UntypedAst>>,
    scopes: Vec<SsaBlock<'ast, UntypedAst>>,
    name_to_id: Vec<HashMap<String, ArgOrVar<'ast, UntypedAst>>>,
}

impl<'ast> ConvertToSsa<'ast> {
    fn new() -> Self {
        Self {
            uid: 0,
            ids: Vec::new(),
            scopes: Vec::new(),
            name_to_id: Vec::new(),
        }
    }

    fn alloc_avis(&self, name: String, ty: MaybeType) -> &'ast Avis<UntypedAst> {
        Box::leak(Box::new(Avis { name, ty }))
    }

    fn alloc_expr(&self, expr: Expr<'ast, UntypedAst>) -> &'ast Expr<'ast, UntypedAst> {
        Box::leak(Box::new(expr))
    }

    fn fresh_uid(&mut self) -> String {
        self.uid += 1;
        format!("_ssa_{}", self.uid)
    }

    pub fn convert_fundef(&mut self, fundef: parse_ast::Fundef) -> Fundef<'ast, UntypedAst> {
        let mut args = Vec::new();
        let mut arg_scope = HashMap::new();

        for (i, (ty, name)) in fundef.args.into_iter().enumerate() {
            let avis = self.alloc_avis(name.clone(), MaybeType(Some(ty)));
            args.push(avis);
            arg_scope.insert(name, ArgOrVar::Arg(i));
        }

        self.ids.clear();
        self.name_to_id = vec![arg_scope, HashMap::new()];
        self.scopes = vec![Vec::new()];

        for stmt in fundef.body {
            self.convert_stmt(stmt);
        }

        let ret = self.convert_expr(fundef.ret_expr);
        let mut body = self.scopes.pop().unwrap();
        body.push(Stmt::Return { id: ret });
        self.name_to_id.clear();

        Fundef {
            name: fundef.id,
            args,
            ids: self.ids.clone(),
            body,
        }
    }

    pub fn convert_stmt(&mut self, stmt: parse_ast::Stmt) {
        match stmt {
            parse_ast::Stmt::Assign { lhs, expr } => {
                let id = self.convert_expr(expr);
                self.name_to_id.last_mut().unwrap().insert(lhs, id);
            }
        }
    }

    pub fn convert_expr(&mut self, expr: parse_ast::Expr) -> ArgOrVar<'ast, UntypedAst> {
        let built = match expr {
            parse_ast::Expr::Tensor { expr, iv, lb, ub } => {
                let lb = self.convert_expr(*lb);
                let ub = self.convert_expr(*ub);

                let iv_avis = self.alloc_avis(iv.clone(), MaybeType(None));
                self.ids.push(iv_avis);

                let mut scope = HashMap::new();
                scope.insert(iv, ArgOrVar::Var(iv_avis));

                self.name_to_id.push(scope);
                self.scopes.push(vec![Stmt::Index {
                    avis: iv_avis,
                    lb,
                    ub,
                }]);
                let ret = self.convert_expr(*expr);
                let ssa = self.scopes.pop().unwrap();
                self.name_to_id.pop().unwrap();

                Expr::Tensor(Tensor { iv: iv_avis, lb, ub, ret, ssa })
            }
            parse_ast::Expr::Binary { l, r, op } => {
                let l = self.convert_expr(*l);
                let r = self.convert_expr(*r);
                Expr::Binary(Binary { l, r, op })
            }
            parse_ast::Expr::Unary { r, op } => {
                let r = self.convert_expr(*r);
                Expr::Unary(Unary { r, op })
            }
            parse_ast::Expr::Bool(v) => Expr::Bool(v),
            parse_ast::Expr::U32(v) => Expr::U32(v),
            parse_ast::Expr::Identifier(id) => {
                for scope in self.name_to_id.iter().rev() {
                    if let Some(v) = scope.get(&id) {
                        return *v;
                    }
                }
                unreachable!("could not find {id}")
            }
        };

        let name = self.fresh_uid();
        let avis = self.alloc_avis(name, MaybeType(None));
        self.ids.push(avis);
        let expr_ref = self.alloc_expr(built);
        self.scopes.last_mut().unwrap().push(Stmt::Assign {
            avis,
            expr: expr_ref,
        });
        ArgOrVar::Var(avis)
    }
}
