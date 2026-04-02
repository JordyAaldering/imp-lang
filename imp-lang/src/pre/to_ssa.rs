use std::collections::HashMap;

use crate::ast::*;

pub fn to_ssa<'ast>(program: Program<'ast, FlattenedAst>) -> Program<'ast, UntypedAst> {
    let fundefs = program
        .fundefs
        .into_iter()
        .map(|f| ToSsa::new().trav_fundef(f))
        .collect();
    Program { fundefs }
}

pub struct ToSsa<'ast> {
    uid: usize,
    ids: Vec<&'ast VarInfo<'ast, UntypedAst>>,
    body_stack: Vec<Vec<Stmt<'ast, UntypedAst>>>,
    env_stack: Vec<HashMap<String, Id<'ast, UntypedAst>>>,
}

impl<'ast> ToSsa<'ast> {
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

    fn alloc_farg(&self, name: String, ty: Type) -> &'ast Farg {
        Box::leak(Box::new(Farg { name, ty }))
    }

    fn alloc_lvis(&self, name: String, ty: Option<Type>, ssa: Option<&'ast Expr<'ast, UntypedAst>>) -> &'ast VarInfo<'ast, UntypedAst> {
        Box::leak(Box::new(VarInfo { name, ty, ssa }))
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

    fn trav_fundef(&mut self, fundef: Fundef<'ast, FlattenedAst>) -> Fundef<'ast, UntypedAst> {
        let mut args = Vec::with_capacity(fundef.args.len());

        for arg in fundef.args {
            args.push(self.alloc_farg(arg.name.clone(), arg.ty.clone()));
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

    fn trav_stmt(&mut self, stmt: Stmt<'ast, FlattenedAst>) {
        match stmt {
            Stmt::Assign(assign) => self.trav_assign(assign),
            Stmt::Return(ret) => self.trav_return(ret),
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, FlattenedAst>) {
        let id = self.trav_expr((*assign.expr).clone());
        self.bind_env(assign.lvis.name.clone(), id);
    }

    fn trav_return(&mut self, ret: Return<'ast, FlattenedAst>) {
        let id = self.trav_id(ret.id);
        self.body_stack
            .last_mut()
            .expect("missing body")
            .push(Stmt::Return(Return { id }));
    }

    fn trav_expr(&mut self, expr: Expr<'ast, FlattenedAst>) -> Id<'ast, UntypedAst> {
        match expr {
            Expr::Id(id) => self.trav_id(id),
            Expr::Tensor(n) => {
                let n = self.trav_tensor(n);
                self.emit_expr(Expr::Tensor(n))
            }
            Expr::Binary(n) => {
                let n = self.trav_binary(n);
                self.emit_expr(Expr::Binary(n))
            }
            Expr::Unary(n) => {
                let n = self.trav_unary(n);
                self.emit_expr(Expr::Unary(n))
            }
            Expr::Bool(v) => self.emit_expr(Expr::Bool(v)),
            Expr::U32(v) => self.emit_expr(Expr::U32(v)),
        }
    }

    fn emit_expr(&mut self, expr: Expr<'ast, UntypedAst>) -> Id<'ast, UntypedAst> {
        let name = self.fresh_uid();
        let expr_ref = self.alloc_expr(expr);
        let lvis = self.alloc_lvis(name, None, Some(expr_ref));
        self.ids.push(lvis);
        self.body_stack
            .last_mut()
            .expect("missing body")
            .push(Stmt::Assign(Assign { lvis, expr: expr_ref }));
        Id::Var(lvis)
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, FlattenedAst>) -> Tensor<'ast, UntypedAst> {
        let lb = self.trav_id(tensor.lb);
        let ub = self.trav_id(tensor.ub);

        let iv_lvis = self.alloc_lvis(tensor.iv.name.clone(), None, None);
        self.ids.push(iv_lvis);

        self.push_env();
        self.bind_env(tensor.iv.name.clone(), Id::Var(iv_lvis));

        self.body_stack.push(Vec::new());
        for stmt in tensor.body {
            self.trav_stmt(stmt);
        }
        let ret = self.trav_id(tensor.ret);
        let body = self.body_stack.pop().expect("missing tensor body");

        self.pop_env();

        Tensor {
            body,
            ret,
            iv: iv_lvis,
            lb,
            ub,
        }
    }

    fn trav_binary(&mut self, binary: Binary<'ast, FlattenedAst>) -> Binary<'ast, UntypedAst> {
        let l = self.trav_id(binary.l);
        let r = self.trav_id(binary.r);
        Binary { l, r, op: binary.op }
    }

    fn trav_unary(&mut self, unary: Unary<'ast, FlattenedAst>) -> Unary<'ast, UntypedAst> {
        let r = self.trav_id(unary.r);
        Unary { r, op: unary.op }
    }

    fn trav_id(&mut self, id: Id<'ast, FlattenedAst>) -> Id<'ast, UntypedAst> {
        match id {
            Id::Arg(i) => Id::Arg(i),
            Id::Var(v) => self
                .lookup_env(&v)
                .unwrap_or_else(|| panic!("could not resolve id {v}")),
        }
    }
}
