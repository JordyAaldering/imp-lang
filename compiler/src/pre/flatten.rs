use std::collections::HashMap;

use crate::ast::*;

pub fn flatten<'ast>(program: Program<'ast, ParsedAst>) -> Program<'ast, FlattenedAst> {
    let fundefs = program
        .fundefs
        .into_iter()
        .map(|f| Flatten::new().trav_fundef(f))
        .collect();
    Program { fundefs }
}

struct Flatten<'ast> {
    uid: usize,
    body_stack: Vec<Vec<Stmt<'ast, FlattenedAst>>>,
    env_stack: Vec<HashMap<String, Id<'ast, FlattenedAst>>>,
}

impl<'ast> Flatten<'ast> {
    fn new() -> Self {
        Self {
            uid: 0,
            body_stack: Vec::new(),
            env_stack: Vec::new(),
        }
    }

    fn fresh_uid(&mut self) -> String {
        self.uid += 1;
        format!("_flat_{}", self.uid)
    }

    fn alloc_avis(&self, name: String, ty: MaybeType) -> &'ast Avis<FlattenedAst> {
        Box::leak(Box::new(Avis { name, ty }))
    }

    fn alloc_expr(&self, expr: Expr<'ast, FlattenedAst>) -> &'ast Expr<'ast, FlattenedAst> {
        Box::leak(Box::new(expr))
    }

    fn push_env(&mut self) {
        self.env_stack.push(HashMap::new());
    }

    fn pop_env(&mut self) {
        self.env_stack.pop().expect("env stack underflow");
    }

    fn bind_env(&mut self, name: String, id: Id<'ast, FlattenedAst>) {
        self.env_stack.last_mut().expect("missing env").insert(name, id);
    }

    fn lookup_env(&self, name: &str) -> Option<Id<'ast, FlattenedAst>> {
        for env in self.env_stack.iter().rev() {
            if let Some(id) = env.get(name) {
                return Some(id.clone());
            }
        }
        None
    }

    fn emit_expr(&mut self, expr: Expr<'ast, FlattenedAst>) -> Id<'ast, FlattenedAst> {
        let name = self.fresh_uid();
        let avis = self.alloc_avis(name.clone(), MaybeType(None));
        let expr = self.alloc_expr(expr);
        self.body_stack
            .last_mut()
            .expect("missing body")
            .push(Stmt::Assign(Assign { avis, expr }));
        let id = Id::Var(name.clone());
        self.bind_env(name, id.clone());
        id
    }

    fn trav_fundef(&mut self, fundef: Fundef<'ast, ParsedAst>) -> Fundef<'ast, FlattenedAst> {
        let mut args = Vec::with_capacity(fundef.args.len());
        for arg in fundef.args {
            args.push(self.alloc_avis(arg.name.clone(), arg.ty.clone()));
        }

        self.push_env();
        for (i, arg) in args.iter().enumerate() {
            self.bind_env(arg.name.clone(), Id::Arg(i));
        }
        self.body_stack.push(Vec::new());

        for stmt in fundef.body {
            self.trav_stmt(stmt);
        }

        let body = self.body_stack.pop().expect("missing function body");
        self.pop_env();

        Fundef {
            name: fundef.name,
            args,
            decs: Vec::new(),
            body,
            ret_type: fundef.ret_type,
        }
    }

    fn trav_stmt(&mut self, stmt: Stmt<'ast, ParsedAst>) {
        match stmt {
            Stmt::Assign(assign) => {
                let rhs = self.trav_expr((*assign.expr).clone());
                let lhs_name = assign.avis.name.clone();
                let lhs_avis = self.alloc_avis(lhs_name.clone(), assign.avis.ty.clone());
                let rhs_expr = self.alloc_expr(Expr::Id(rhs));
                self.body_stack
                    .last_mut()
                    .expect("missing body")
                    .push(Stmt::Assign(Assign {
                        avis: lhs_avis,
                        expr: rhs_expr,
                    }));
                self.bind_env(lhs_name.clone(), Id::Var(lhs_name));
            }
            Stmt::Return(ret) => {
                let id = self.trav_id(ret.id);
                self.body_stack
                    .last_mut()
                    .expect("missing body")
                    .push(Stmt::Return(Return { id }));
            }
        }
    }

    fn trav_expr(&mut self, expr: Expr<'ast, ParsedAst>) -> Id<'ast, FlattenedAst> {
        match expr {
            Expr::Id(id) => self.trav_id(id),
            Expr::Bool(v) => self.emit_expr(Expr::Bool(v)),
            Expr::U32(v) => self.emit_expr(Expr::U32(v)),
            Expr::Unary(unary) => {
                let r = self.trav_expr((*unary.r).clone());
                self.emit_expr(Expr::Unary(Unary { r, op: unary.op }))
            }
            Expr::Binary(binary) => {
                let l = self.trav_expr((*binary.l).clone());
                let r = self.trav_expr((*binary.r).clone());
                self.emit_expr(Expr::Binary(Binary {
                    l,
                    r,
                    op: binary.op,
                }))
            }
            Expr::Tensor(tensor) => {
                let lb = self.trav_expr((*tensor.lb).clone());
                let ub = self.trav_expr((*tensor.ub).clone());

                self.push_env();
                self.bind_env(tensor.iv.name.clone(), Id::Var(tensor.iv.name.clone()));

                self.body_stack.push(Vec::new());
                for stmt in tensor.body {
                    self.trav_stmt(stmt);
                }
                let ret = self.trav_expr((*tensor.ret).clone());
                let body = self.body_stack.pop().expect("missing tensor body");

                self.pop_env();

                let iv = self.alloc_avis(tensor.iv.name.clone(), tensor.iv.ty.clone());
                self.emit_expr(Expr::Tensor(Tensor {
                    body,
                    ret,
                    iv,
                    lb,
                    ub,
                }))
            }
        }
    }

    fn trav_id(&mut self, id: Id<'ast, ParsedAst>) -> Id<'ast, FlattenedAst> {
        match id {
            Id::Arg(i) => Id::Arg(i),
            Id::Var(name) => self.lookup_env(&name).unwrap_or(Id::Var(name)),
        }
    }
}
