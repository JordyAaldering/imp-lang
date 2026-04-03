use std::collections::HashMap;

use crate::ast::*;

pub fn flatten<'ast>(program: Program<'ast, ParsedAst>) -> Program<'ast, FlattenedAst> {
    let fundefs = program
        .fundefs
        .into_iter()
        .map(|(name, wrapper)| {
            let overloads = wrapper
                .overloads
                .into_iter()
                .map(|f| Flatten::new().trav_fundef(f))
                .collect();
            (name, FundefWrapper { name: wrapper.name, overloads })
        })
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

    fn alloc_farg(&self, name: String, ty: Type) -> &'ast Farg {
        Box::leak(Box::new(Farg { name, ty }))
    }

    fn alloc_lvis(&self, name: String, ty: Option<Type>) -> &'ast VarInfo<'ast, FlattenedAst> {
        Box::leak(Box::new(VarInfo { name, ty, ssa: () }))
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
        let lvis = self.alloc_lvis(name.clone(), None);
        let expr = self.alloc_expr(expr);
        self.body_stack
            .last_mut()
            .expect("missing body")
            .push(Stmt::Assign(Assign { lvis, expr }));
        let id = Id::Var(name.clone());
        self.bind_env(name, id.clone());
        id
    }

    fn trav_fundef(&mut self, fundef: Fundef<'ast, ParsedAst>) -> Fundef<'ast, FlattenedAst> {
        let mut args = Vec::with_capacity(fundef.args.len());
        for arg in fundef.args {
            args.push(self.alloc_farg(arg.name.clone(), arg.ty.clone()));
        }

        self.push_env();
        for (i, arg) in args.iter().enumerate() {
            self.bind_env(arg.name.clone(), Id::Arg(i));
        }
        self.body_stack.push(Vec::new());

        // Bind `d` and `shp` from `d:shp` rank captures directly in the env.
        // Also bind dimension variables from regular Dim type patterns (e.g. `usize[d]`).
        // No assignment stmts are injected — these are pure projections of the farg resolved at codegen.
        for (i, arg) in args.iter().enumerate() {
            if let ShapePattern::Axes(axes) = &arg.ty.shape {
                for (k, axis) in axes.iter().enumerate() {
                    if let AxisPattern::Rank(capture) = axis {
                        self.bind_env(capture.dim_name.clone(), Id::Dim(i));
                        self.bind_env(capture.shp_name.clone(), Id::Shp(i));
                    } else if let AxisPattern::Dim(DimPattern::Var(var)) = axis
                        && var.role == SymbolRole::Define {
                        self.bind_env(var.name.clone(), Id::DimAt(i, k));
                    }
                }
            }
        }

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
                let lhs_name = assign.lvis.name.clone();
                let lhs_lvis = self.alloc_lvis(lhs_name.clone(), assign.lvis.ty.clone());
                let rhs_expr = self.alloc_expr(Expr::Id(rhs));
                self.body_stack
                    .last_mut()
                    .expect("missing body")
                    .push(Stmt::Assign(Assign {
                        lvis: lhs_lvis,
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
            Expr::Call(call) => {
                let call = self.trav_call(call);
                self.emit_expr(Expr::Call(call))
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

                let iv = self.alloc_lvis(tensor.iv.name.clone(), tensor.iv.ty.clone());
                self.emit_expr(Expr::Tensor(Tensor {
                    body,
                    ret,
                    iv,
                    lb,
                    ub,
                }))
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
            Expr::Unary(unary) => {
                let r = self.trav_expr((*unary.r).clone());
                self.emit_expr(Expr::Unary(Unary { r, op: unary.op }))
            }
            Expr::Array(array) => {
                let mut values = Vec::with_capacity(array.values.len());
                for value in array.values {
                    values.push(self.trav_expr(value.clone()));
                }
                self.emit_expr(Expr::Array(Array { values }))
            }
            Expr::Sel(sel) => {
                let arr = self.trav_expr((*sel.arr).clone());
                let idx = self.trav_expr((*sel.idx).clone());
                self.emit_expr(Expr::Sel(Sel { arr, idx }))
            }
            Expr::Id(id) => self.trav_id(id),
            Expr::Bool(v) => self.emit_expr(Expr::Bool(v)),
            Expr::U32(v) => self.emit_expr(Expr::U32(v)),
        }
    }

    fn trav_call(&mut self, call: Call<'ast, ParsedAst>) -> Call<'ast, FlattenedAst> {
        let mut args = Vec::with_capacity(call.args.len());
        for arg in call.args {
            args.push(self.trav_expr(arg.clone()));
        }

        Call {
            id: call.id,
            args,
        }
    }

    fn trav_id(&mut self, id: Id<'ast, ParsedAst>) -> Id<'ast, FlattenedAst> {
        match id {
            Id::Arg(i) => Id::Arg(i),
            Id::Var(name) => self.lookup_env(&name).unwrap_or(Id::Var(name)),
            Id::Dim(i) => Id::Dim(i),
            Id::Shp(i) => Id::Shp(i),
            Id::DimAt(i, k) => Id::DimAt(i, k),
        }
    }
}
