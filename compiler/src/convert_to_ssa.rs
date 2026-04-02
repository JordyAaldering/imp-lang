use std::collections::HashMap;

use crate::{ast::*, scanparse::parse_ast};

pub fn convert_to_ssa<'ast>(program: parse_ast::Program) -> Program<'ast, UntypedAst> {
    let fundefs = program.fundefs.into_iter()
        .map(|f| ConvertToSsa::new().trav_fundef(f))
        .collect();
    Program { fundefs }
}

pub struct ConvertToSsa<'ast> {
    uid: usize,
    ids: Vec<&'ast Avis<UntypedAst>>,
    body_stack: Vec<Vec<Stmt<'ast, UntypedAst>>>,
    name_to_id: Vec<HashMap<String, Id<'ast, UntypedAst>>>,
}

impl<'ast> ConvertToSsa<'ast> {
    fn new() -> Self {
        Self {
            uid: 0,
            ids: Vec::new(),
            body_stack: Vec::new(),
            name_to_id: Vec::new(),
        }
    }

    fn fresh_uid(&mut self) -> String {
        self.uid += 1;
        format!("_ssa_{}", self.uid)
    }

    fn alloc_avis(&self, name: String, ty: MaybeType) -> &'ast Avis<UntypedAst> {
        Box::leak(Box::new(Avis { name, ty }))
    }

    fn alloc_expr(&self, expr: Expr<'ast, UntypedAst>) -> &'ast Expr<'ast, UntypedAst> {
        Box::leak(Box::new(expr))
    }

    ///
    /// Declarations
    ///

    fn trav_fundef(&mut self, fundef: parse_ast::Fundef) -> Fundef<'ast, UntypedAst> {
        let mut args = Vec::new();
        let mut arg_scope = HashMap::new();

        for (i, (ty, name)) in fundef.args.into_iter().enumerate() {
            args.push(self.alloc_avis(name.clone(), MaybeType(Some(ty))));
            arg_scope.insert(name, Id::Arg(i));
        }

        self.name_to_id = vec![arg_scope, HashMap::new()];
        self.body_stack = vec![Vec::new()];

        for stmt in fundef.body {
            self.trav_stmt(stmt);
        }

        let body = self.body_stack.pop().unwrap();

        Fundef {
            name: fundef.id,
            args,
            decs: self.ids.clone(),
            body,
        }
    }

    ///
    /// Statements
    ///

    fn trav_stmt(&mut self, stmt: parse_ast::Stmt) {
        use parse_ast::Stmt::*;
        match stmt {
            Assign { lhs, expr } => self.trav_assign(lhs, expr),
            Return { expr } => self.trav_return(expr),
        }
    }

    fn trav_assign(&mut self, lhs: String, expr: parse_ast::Expr) {
        let id = self.trav_expr(expr);
        self.name_to_id.last_mut().unwrap().insert(lhs, id);
    }

    fn trav_return(&mut self, expr: parse_ast::Expr) {
        let id = self.trav_expr(expr);
        self.body_stack.last_mut().unwrap().push(Stmt::Return(Return { id }));
    }

    ///
    /// Expressions
    ///

    fn trav_expr(&mut self, expr: parse_ast::Expr) -> Id<'ast, UntypedAst> {
        use parse_ast::Expr::*;
        match expr {
            Id(id) => self.trav_id(id),
            expr => {
                let expr = match expr {
                    Tensor { expr, iv, lb, ub } => self.trav_tensor_expr(*expr, iv, *lb, *ub),
                    Binary { l, r, op } => self.trav_binary(*l, *r, op),
                    Unary { r, op } => self.trav_unary(*r, op),
                    Bool(v) => Expr::Bool(v),
                    U32(v) => Expr::U32(v),
                    Id(_) => unreachable!(),
                };
                self.emit_expr(expr)
            }
        }
    }

    fn emit_expr(&mut self, expr: Expr<'ast, UntypedAst>) -> Id<'ast, UntypedAst> {
        let name = self.fresh_uid();
        let avis = self.alloc_avis(name, MaybeType(None));
        self.ids.push(avis);
        let expr_ref = self.alloc_expr(expr);
        self.body_stack.last_mut().unwrap().push(Stmt::Assign(Assign { avis, expr: expr_ref }));
        Id::Var(avis)
    }

    fn trav_tensor_expr(&mut self, expr: parse_ast::Expr, iv: String, lb: parse_ast::Expr, ub: parse_ast::Expr) -> Expr<'ast, UntypedAst> {
        let lb = self.trav_expr(lb);
        let ub = self.trav_expr(ub);

        let iv_avis = self.alloc_avis(iv.clone(), MaybeType(None));
        self.ids.push(iv_avis);

        let mut scope = HashMap::new();
        scope.insert(iv, Id::Var(iv_avis));

        self.name_to_id.push(scope);
        self.body_stack.push(Vec::new());
        let ret = self.trav_expr(expr);
        let body = self.body_stack.pop().unwrap();
        self.name_to_id.pop().unwrap();

        Expr::Tensor(Tensor {
            iv: iv_avis,
            lb,
            ub,
            ret,
            body,
        })
    }

    fn trav_binary(&mut self, l: parse_ast::Expr, r: parse_ast::Expr, op: Bop) -> Expr<'ast, UntypedAst> {
        let l = self.trav_expr(l);
        let r = self.trav_expr(r);
        Expr::Binary(Binary { l, r, op })
    }

    fn trav_unary(&mut self, r: parse_ast::Expr, op: Uop) -> Expr<'ast, UntypedAst> {
        let r = self.trav_expr(r);
        Expr::Unary(Unary { r, op })
    }

    ///
    /// Terminals
    ///

    fn trav_id(&mut self, id: String) -> Id<'ast, UntypedAst> {
        for scope in self.name_to_id.iter().rev() {
            if let Some(v) = scope.get(&id) {
                return *v;
            }
        }
        unreachable!("could not find {id}")
    }
}
