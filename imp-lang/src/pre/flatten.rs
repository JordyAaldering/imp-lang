use std::{collections::HashMap, mem};

use crate::{ast::*, traverse::Traverse};

pub fn flatten<'ast>(program: Program<'ast, ParsedAst>) -> Program<'ast, FlattenedAst> {
    Flatten::new().trav_program(program)
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

        self.new_assigns.push(Stmt::Assign(Assign { lhs: lvis, expr }));

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
            self.bind_env(arg.id.clone(), Id::Arg(i));
        }

        let mut shape_prelude = Vec::new();
        for assign in fundef.shape_prelude {
            let assign = self.trav_assign(assign);
            let new_assigns = mem::take(&mut self.new_assigns);
            for stmt in new_assigns {
                match stmt {
                    Stmt::Assign(assign) => shape_prelude.push(assign),
                }
            }
            shape_prelude.push(assign);
        }

        let body = self.trav_body(fundef.body);

        self.pop_env();

        Fundef {
            name: fundef.name,
            args: fundef.args,
            shape_prelude,
            shape_facts: fundef.shape_facts,
            decs: Vec::new(),
            body,
            ret_type: fundef.ret_type,
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst> {
        let rhs = self.trav_expr((*assign.expr).clone());
        let lhs_name = assign.lhs.name.clone();
        let lhs_lvis = self.alloc_lvis(lhs_name.clone(), assign.lhs.ty.clone());
        let rhs_expr = self.alloc_expr(Expr::Id(rhs));

        self.bind_env(lhs_name.clone(), Id::Var(lhs_name));

        Assign { lhs: lhs_lvis, expr: rhs_expr }
    }

    fn trav_body(&mut self, body: Body<'ast, Self::InAst>) -> Body<'ast, Self::OutAst> {
        let old_assigns = mem::take(&mut self.new_assigns);

        let mut stmts = Vec::new();

        for stmt in body.stmts {
            let stmt = self.trav_stmt(stmt);
            stmts.extend(mem::take(&mut self.new_assigns));
            stmts.push(stmt);
        }

        let ret = self.trav_expr((*body.ret).clone());
        stmts.extend(mem::take(&mut self.new_assigns));

        self.new_assigns = old_assigns;
        Body { stmts, ret }
    }

    type ExprOut = Id<'ast, Self::OutAst>;

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
        use Expr::*;
        let expr = match expr {
            Cond(n) => Cond(self.trav_cond(n)),
            Call(n) => Call(self.trav_call(n)),
            PrfCall(n) => PrfCall(self.trav_prf_call(n)),
            Fold(n) => Fold(self.trav_fold(n)),
            Tensor(n) => Tensor(self.trav_tensor(n)),
            Array(n) => Array(self.trav_array(n)),
            Id(n) => Id(self.trav_id(n)),
            Const(c) => Const(c),
        };
        self.emit_expr(expr)
    }

    fn trav_cond(&mut self, cond: Cond<'ast, Self::InAst>) -> Self::CondOut {
        let c = self.trav_expr(cond.cond.clone());
        let t = self.trav_body(cond.then_branch);
        let e = self.trav_body(cond.else_branch);
        Cond { cond: c, then_branch: t, else_branch: e }
    }

    fn trav_call(&mut self, call: Call<'ast, Self::InAst>) -> Self::CallOut {
        let mut args = Vec::with_capacity(call.args.len());
        for arg in call.args {
            args.push(self.trav_expr(arg.clone()));
        }

        Call { id: call.id, args }
    }

    fn trav_prf_call(&mut self, prf: PrfCall<'ast, Self::InAst>) -> Self::PrfCallOut {
        use PrfCall::*;
        match prf {
            ShapeA(a) => {
                let a = self.trav_expr(a.clone());
                ShapeA(a)
            }
            DimA(a) => {
                let a = self.trav_expr(a.clone());
                DimA(a)
            }
            AddSxS(l, r) => {
                let l = self.trav_expr(l.clone());
                let r = self.trav_expr(r.clone());
                AddSxS(l, r)
            }
            SubSxS(l, r) => {
                let l = self.trav_expr(l.clone());
                let r = self.trav_expr(r.clone());
                SubSxS(l, r)
            }
            MulSxS(l, r) => {
                let l = self.trav_expr(l.clone());
                let r = self.trav_expr(r.clone());
                MulSxS(l, r)
            }
            DivSxS(l, r) => {
                let l = self.trav_expr(l.clone());
                let r = self.trav_expr(r.clone());
                DivSxS(l, r)
            }
            SelVxA(a, b) => {
                let a = self.trav_expr(a.clone());
                let b = self.trav_expr(b.clone());
                SelVxA(a, b)
            }
            LtSxS(a, b) => {
                let a = self.trav_expr(a.clone());
                let b = self.trav_expr(b.clone());
                LtSxS(a, b)
            }
            LeSxS(a, b) => {
                let a = self.trav_expr(a.clone());
                let b = self.trav_expr(b.clone());
                LeSxS(a, b)
            }
            GtSxS(a, b) => {
                let a = self.trav_expr(a.clone());
                let b = self.trav_expr(b.clone());
                GtSxS(a, b)
            }
            GeSxS(a, b) => {
                let a = self.trav_expr(a.clone());
                let b = self.trav_expr(b.clone());
                GeSxS(a, b)
            }
            EqSxS(a, b) => {
                let a = self.trav_expr(a.clone());
                let b = self.trav_expr(b.clone());
                EqSxS(a, b)
            }
            NeSxS(a, b) => {
                let a = self.trav_expr(a.clone());
                let b = self.trav_expr(b.clone());
                NeSxS(a, b)
            }
            NegS(a) => {
                let a = self.trav_expr(a.clone());
                NegS(a)
            }
            NotS(a) => {
                let a = self.trav_expr(a.clone());
                NotS(a)
            }
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
        let lb = if let Some(lb) = tensor.lb {
            Some(self.trav_expr(lb.clone()))
        } else {
            None
        };

        let ub = self.trav_expr((*tensor.ub).clone());

        self.push_env();
        self.bind_env(tensor.iv.name.clone(), Id::Var(tensor.iv.name.clone()));

        let body = self.trav_body(tensor.body);

        self.pop_env();

        let iv = self.alloc_lvis(tensor.iv.name.clone(), tensor.iv.ty.clone());

        Tensor { body, iv, lb, ub }
    }

    fn trav_fold(&mut self, fold: Fold<'ast, Self::InAst>) -> Self::FoldOut {
        let neutral = self.trav_expr((*fold.neutral).clone());

        let foldfun = match fold.foldfun {
            FoldFun::Name(id) => FoldFun::Name(id),
            FoldFun::Apply { id, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| match arg {
                        FoldFunArg::Placeholder => FoldFunArg::Placeholder,
                        FoldFunArg::Bound(bound) => {
                            FoldFunArg::Bound(self.trav_expr((*bound).clone()))
                        }
                    })
                    .collect();
                FoldFun::Apply { id, args }
            }
        };

        let selection = self.trav_tensor(fold.selection);

        Fold {
            neutral,
            foldfun,
            selection,
        }
    }

    fn trav_array(&mut self, array: Array<'ast, Self::InAst>) -> Self::ArrayOut {
        let mut values = Vec::with_capacity(array.elems.len());
        for value in array.elems {
            values.push(self.trav_expr(value.clone()));
        }
        Array { elems: values }
    }

    fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Self::IdOut {
        match id {
            Id::Arg(i) => Id::Arg(i),
            Id::Var(name) => self.lookup_env(&name).unwrap_or(Id::Var(name)),
        }
    }
}
