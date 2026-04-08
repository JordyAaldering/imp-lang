use std::{collections::HashMap, mem};

use crate::{ast::*, traverse::Traverse};

pub fn flatten<'ast>(program: Program<'ast, ParsedAst>) -> Program<'ast, FlattenedAst> {
    let functions = program.functions
        .into_iter()
        .map(|(name, fundef)| {
            (name, Flatten::new().trav_fundef(fundef))
        })
        .collect();
    Program {
        functions,
        typesets: program.typesets,
        members: program.members,
        traits: program.traits,
        impls: program.impls,
    }
}

struct Flatten<'ast> {
    uid: usize,
    new_assigns: Vec<Stmt<'ast, FlattenedAst>>,
    env_stack: Vec<HashMap<String, Id<'ast, FlattenedAst>>>,
}

impl<'ast> Flatten<'ast> {
    fn new() -> Self {
        Self {
            uid: 0,
            new_assigns: Vec::new(),
            env_stack: Vec::new(),
        }
    }

    fn fresh_uid(&mut self) -> String {
        self.uid += 1;
        format!("_flat_{}", self.uid)
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
        self.env_stack.pop().unwrap();
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

        self.new_assigns.push(Stmt::Assign(Assign { lvis, expr }));

        let id = Id::Var(name.clone());
        self.bind_env(name, id.clone());
        id
    }
}

impl<'ast> Traverse<'ast> for Flatten<'ast> {
    type InAst = ParsedAst;

    type OutAst = FlattenedAst;

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        self.push_env();

        for (i, arg) in fundef.args.iter().enumerate() {
            self.bind_env(arg.name.clone(), Id::Arg(i));
        }

        for (i, arg) in fundef.args.iter().enumerate() {
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

        let mut body = Vec::new();
        for stmt in fundef.body {
            let stmt = self.trav_stmt(stmt);
            body.extend(mem::take(&mut self.new_assigns));
            body.push(stmt);
        }

        self.pop_env();

        Fundef {
            is_public: fundef.is_public,
            name: fundef.name,
            args: fundef.args,
            decs: Vec::new(),
            body,
            ret_type: fundef.ret_type,
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst> {
        let rhs = self.trav_expr((*assign.expr).clone());
        let lhs_name = assign.lvis.name.clone();
        let lhs_lvis = self.alloc_lvis(lhs_name.clone(), assign.lvis.ty.clone());
        let rhs_expr = self.alloc_expr(Expr::Id(rhs));

        self.bind_env(lhs_name.clone(), Id::Var(lhs_name));

        Assign { lvis: lhs_lvis, expr: rhs_expr }
    }

    fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Return<'ast, Self::OutAst> {
        let id = self.trav_id(ret.id);
        Return { id }
    }

    type ExprOut = Id<'ast, Self::OutAst>;

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
        use Expr::*;
        let expr = match expr {
            Call(n) => Call(self.trav_call(n)),
            PrfCall(n) => PrfCall(self.trav_prf_call(n)),
            Tensor(n) => Tensor(self.trav_tensor(n)),
            Array(n) => Array(self.trav_array(n)),
            Id(n) => Id(self.trav_id(n)),
            I32(v) => I32(v),
            I64(v) => I64(v),
            U32(v) => U32(v),
            U64(v) => U64(v),
            Usize(v) => Usize(v),
            F32(v) => F32(v),
            F64(v) => F64(v),
            Bool(v) => Bool(v),
        };
        self.emit_expr(expr)
    }

    fn trav_call(&mut self, call: Call<'ast, Self::InAst>) -> Self::CallOut {
        let mut args = Vec::with_capacity(call.args.len());
        for arg in call.args {
            args.push(self.trav_expr(arg.clone()));
        }

        Call { id: call.id, args }
    }

    fn trav_prf_call(&mut self, prf_call: PrfCall<'ast, Self::InAst>) -> Self::PrfCallOut {
        let mut args = Vec::with_capacity(prf_call.args.len());
        for arg in prf_call.args {
            args.push(self.trav_expr(arg.clone()));
        }

        PrfCall { id: prf_call.id, args }
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
        let lb = self.trav_expr((*tensor.lb).clone());
        let ub = self.trav_expr((*tensor.ub).clone());

        self.push_env();
        self.bind_env(tensor.iv.name.clone(), Id::Var(tensor.iv.name.clone()));
        let old_assigns = mem::take(&mut self.new_assigns);

        let mut body = Vec::new();
        for stmt in tensor.body {
            let stmt = self.trav_stmt(stmt);
            body.extend(mem::take(&mut self.new_assigns));
            body.push(stmt);
        }

        let ret = self.trav_expr((*tensor.ret).clone());
        body.extend(mem::take(&mut self.new_assigns));

        self.new_assigns = old_assigns;
        self.pop_env();

        let iv = self.alloc_lvis(tensor.iv.name.clone(), tensor.iv.ty.clone());

        Tensor { body, ret, iv, lb, ub }
    }

    fn trav_array(&mut self, array: Array<'ast, Self::InAst>) -> Self::ArrayOut {
        let mut values = Vec::with_capacity(array.values.len());
        for value in array.values {
            values.push(self.trav_expr(value.clone()));
        }
        Array { values }
    }

    fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Self::IdOut {
        match id {
            Id::Arg(i) => Id::Arg(i),
            Id::Var(name) => self.lookup_env(&name).unwrap_or(Id::Var(name)),
            Id::Dim(i) => Id::Dim(i),
            Id::Shp(i) => Id::Shp(i),
            Id::DimAt(i, k) => Id::DimAt(i, k),
        }
    }
}
