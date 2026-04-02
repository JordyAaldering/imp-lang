use std::collections::HashMap;

use crate::ast::*;

pub fn convert_to_ssa<'ast>(program: Program<'ast, ParseAst>) -> Program<'ast, UntypedAst> {
    let fundefs = program
        .fundefs
        .into_iter()
        .map(|f| ConvertToSsa::new().trav_fundef(f))
        .collect();
    Program { fundefs }
}

pub struct ConvertToSsa<'ast> {
    uid: usize,
    ids: Vec<&'ast Avis<UntypedAst>>,
    body_stack: Vec<Vec<Stmt<'ast, UntypedAst>>>,
    env_stack: Vec<HashMap<String, Id<'ast, UntypedAst>>>,
}

impl<'ast> ConvertToSsa<'ast> {
    fn new() -> Self {
        Self {
            uid: 0,
            ids: Vec::new(),
            body_stack: Vec::new(),
            env_stack: Vec::new(),
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

    fn push_env(&mut self) {
        self.env_stack.push(HashMap::new());
    }

    fn pop_env(&mut self) {
        self.env_stack.pop().expect("env stack underflow");
    }

    fn bind_env(&mut self, name: String, id: Id<'ast, UntypedAst>) {
        self.env_stack.last_mut().expect("missing env").insert(name, id);
    }

    fn lookup_env(&self, name: &str) -> Option<Id<'ast, UntypedAst>> {
        for env in self.env_stack.iter().rev() {
            if let Some(id) = env.get(name) {
                return Some(id.clone());
            }
        }
        None
    }

    fn trav_fundef(&mut self, fundef: Fundef<'ast, ParseAst>) -> Fundef<'ast, UntypedAst> {
        let mut args = Vec::with_capacity(fundef.args.len());

        for arg in fundef.args {
            args.push(self.alloc_avis(arg.name.clone(), arg.ty.clone()));
        }

        self.push_env();
        for (i, arg) in args.iter().enumerate() {
            self.bind_env(arg.name.clone(), Id::Arg(i));
        }
        self.body_stack = vec![Vec::new()];

        for stmt in fundef.body {
            self.trav_stmt(stmt);
        }

        let body = self.body_stack.pop().expect("missing body stack");
        self.pop_env();

        Fundef {
            name: fundef.name,
            args,
            decs: self.ids.clone(),
            body,
            ret_type: fundef.ret_type,
        }
    }

    fn trav_stmt(&mut self, stmt: Stmt<'ast, ParseAst>) {
        match stmt {
            Stmt::Assign(assign) => self.trav_assign(assign),
            Stmt::Return(ret) => self.trav_return(ret),
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, ParseAst>) {
        let id = self.trav_expr((*assign.expr).clone());
        self.bind_env(assign.avis.name.clone(), id);
    }

    fn trav_return(&mut self, ret: Return<'ast, ParseAst>) {
        let id = self.trav_id(ret.id);
        self.body_stack
            .last_mut()
            .expect("missing body")
            .push(Stmt::Return(Return { id }));
    }

    fn trav_expr(&mut self, expr: Expr<'ast, ParseAst>) -> Id<'ast, UntypedAst> {
        match expr {
            Expr::Id(id) => self.trav_id(id),
            other => {
                let built = match other {
                    Expr::Tensor(n) => self.trav_tensor(n),
                    Expr::Binary(n) => self.trav_binary(n),
                    Expr::Unary(n) => self.trav_unary(n),
                    Expr::Bool(v) => Expr::Bool(v),
                    Expr::U32(v) => Expr::U32(v),
                    Expr::Id(_) => unreachable!(),
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
        self.body_stack
            .last_mut()
            .expect("missing body")
            .push(Stmt::Assign(Assign { avis, expr: expr_ref }));
        Id::Var(avis)
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, ParseAst>) -> Expr<'ast, UntypedAst> {
        let lb = self.trav_expr((*tensor.lb).clone());
        let ub = self.trav_expr((*tensor.ub).clone());

        let iv_avis = self.alloc_avis(tensor.iv.name.clone(), MaybeType(None));
        self.ids.push(iv_avis);

        self.push_env();
        self.bind_env(tensor.iv.name.clone(), Id::Var(iv_avis));

        self.body_stack.push(Vec::new());
        for stmt in tensor.body {
            self.trav_stmt(stmt);
        }
        let ret = self.trav_expr((*tensor.ret).clone());
        let body = self.body_stack.pop().expect("missing tensor body");

        self.pop_env();

        Expr::Tensor(Tensor {
            body,
            ret,
            iv: iv_avis,
            lb,
            ub,
        })
    }

    fn trav_binary(&mut self, binary: Binary<'ast, ParseAst>) -> Expr<'ast, UntypedAst> {
        let l = self.trav_expr((*binary.l).clone());
        let r = self.trav_expr((*binary.r).clone());
        Expr::Binary(Binary { l, r, op: binary.op })
    }

    fn trav_unary(&mut self, unary: Unary<'ast, ParseAst>) -> Expr<'ast, UntypedAst> {
        let r = self.trav_expr((*unary.r).clone());
        Expr::Unary(Unary { r, op: unary.op })
    }

    fn trav_id(&mut self, id: Id<'ast, ParseAst>) -> Id<'ast, UntypedAst> {
        match id {
            Id::Arg(i) => Id::Arg(i),
            Id::Var(v) => self
                .lookup_env(&v)
                .unwrap_or_else(|| panic!("could not resolve id {v}")),
        }
    }
}
