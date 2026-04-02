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
            ret_type: MaybeType(Some(fundef.ret_type)),
        }
    }

    ///
    /// Statements
    ///

    fn trav_stmt(&mut self, stmt: parse_ast::Stmt) {
        match stmt {
            parse_ast::Stmt::Assign(assign) => self.trav_assign(assign),
            parse_ast::Stmt::Return(ret) => self.trav_return(ret),
        }
    }

    fn trav_assign(&mut self, assign: parse_ast::Assign) {
        let id = self.trav_expr(assign.expr);
        self.name_to_id.last_mut().unwrap().insert(assign.lhs, id);
    }

    fn trav_return(&mut self, ret: parse_ast::Return) {
        let id = self.trav_expr(ret.expr);
        self.body_stack.last_mut().unwrap().push(Stmt::Return(Return { id }));
    }

    ///
    /// Expressions
    ///

    fn trav_expr(&mut self, expr: parse_ast::Expr) -> Id<'ast, UntypedAst> {
        match expr {
            parse_ast::Expr::Id(id) => self.trav_id(id),
            other => {
                let built = match other {
                    parse_ast::Expr::Tensor(n) => self.trav_tensor(n),
                    parse_ast::Expr::Binary(n) => self.trav_binary(n),
                    parse_ast::Expr::Unary(n) => self.trav_unary(n),
                    parse_ast::Expr::Bool(v) => Expr::Bool(v),
                    parse_ast::Expr::U32(v) => Expr::U32(v),
                    parse_ast::Expr::Id(_) => unreachable!(),
                };
                self.emit_expr(built)
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

    fn trav_tensor(&mut self, tensor: parse_ast::Tensor) -> Expr<'ast, UntypedAst> {
        let lb = self.trav_expr(*tensor.lb);
        let ub = self.trav_expr(*tensor.ub);

        let iv_avis = self.alloc_avis(tensor.iv.clone(), MaybeType(None));
        self.ids.push(iv_avis);

        let mut scope = HashMap::new();
        scope.insert(tensor.iv, Id::Var(iv_avis));

        self.name_to_id.push(scope);
        self.body_stack.push(Vec::new());
        let ret = self.trav_expr(*tensor.expr);
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

    fn trav_binary(&mut self, binary: parse_ast::Binary) -> Expr<'ast, UntypedAst> {
        let l = self.trav_expr(*binary.l);
        let r = self.trav_expr(*binary.r);
        Expr::Binary(Binary { l, r, op: binary.op })
    }

    fn trav_unary(&mut self, unary: parse_ast::Unary) -> Expr<'ast, UntypedAst> {
        let r = self.trav_expr(*unary.r);
        Expr::Unary(Unary { r, op: unary.op })
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
