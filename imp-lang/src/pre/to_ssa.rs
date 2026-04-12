use std::{collections::HashMap, mem};

use crate::{ast::*, traverse::Traverse};

pub fn to_ssa<'ast>(program: Program<'ast, FlattenedAst>) -> Program<'ast, UntypedAst> {
    ToSsa::new().trav_program(program)
}

pub struct ToSsa<'ast> {
    uid: usize,
    decs: Vec<&'ast VarInfo<'ast, UntypedAst>>,
    new_assigns: Vec<Stmt<'ast, UntypedAst>>,
    env_stack: Vec<HashMap<String, Id<'ast, UntypedAst>>>,
}

impl<'ast> ToSsa<'ast> {
    fn new() -> Self {
        Self {
            uid: 0,
            decs: Vec::new(),
            new_assigns: Vec::new(),
            env_stack: Vec::new(),
        }
    }

    fn fresh_uid(&mut self) -> String {
        self.uid += 1;
        format!("_ssa_{}", self.uid)
    }

    fn alloc_lvis(&self, name: String, ssa: Option<&'ast Expr<'ast, UntypedAst>>) -> &'ast VarInfo<'ast, UntypedAst> {
        Box::leak(Box::new(VarInfo { name, ty: None, ssa }))
    }

    fn alloc_expr(&self, expr: Expr<'ast, UntypedAst>) -> &'ast Expr<'ast, UntypedAst> {
        Box::leak(Box::new(expr))
    }

    fn push_env(&mut self) {
        self.env_stack.push(HashMap::new());
    }

    fn pop_env(&mut self) {
        self.env_stack.pop().unwrap();
    }

    fn bind_env(&mut self, name: String, id: Id<'ast, UntypedAst>) {
        self.env_stack.last_mut().unwrap().insert(name, id);
    }

    fn lookup_env(&self, name: &str) -> Option<Id<'ast, UntypedAst>> {
        for env in self.env_stack.iter().rev() {
            if let Some(id) = env.get(name) {
                return Some(*id);
            }
        }
        None
    }
}

impl<'ast> Traverse<'ast> for ToSsa<'ast> {
    type InAst = FlattenedAst;

    type OutAst = UntypedAst;

    fn trav_fundef(&mut self, fundef: Fundef<'ast, FlattenedAst>) -> Fundef<'ast, UntypedAst> {
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
                    Stmt::Return(_) => unreachable!("shape prelude SSA lowering emitted return"),
                }
            }
            shape_prelude.push(assign);
        }

        let mut body = Vec::new();
        for stmt in fundef.body {
            let stmt = self.trav_stmt(stmt);
            body.extend(mem::take(&mut self.new_assigns));
            body.push(stmt);
        }

        self.pop_env();

        Fundef {
            name: fundef.name,
            args: fundef.args,
            shape_prelude,
            shape_facts: fundef.shape_facts,
            decs: mem::take(&mut self.decs),
            body,
            ret_type: fundef.ret_type,
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst> {
        let old_name = assign.lhs.name.clone();
        let new_name = self.fresh_uid();

        let expr = self.trav_expr((*assign.expr).clone());
        let expr = self.alloc_expr(expr);
        let lvis = self.alloc_lvis(new_name, Some(expr));
        self.bind_env(old_name, Id::Var(lvis));
        self.decs.push(lvis);

        Assign { lhs: lvis, expr }
    }

    fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Return<'ast, Self::OutAst> {
        let id = self.trav_id(ret.id);
        Return { id }
    }

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
        use Expr::*;
        match expr {
            Cond(n) => Cond(self.trav_cond(n)),
            Call(n) => Call(self.trav_call(n)),
            PrfCall(n) => PrfCall(self.trav_prf_call(n)),
            Fold(n) => Fold(self.trav_fold(n)),
            Tensor(n) => Tensor(self.trav_tensor(n)),
            Array(n) => Array(self.trav_array(n)),
            Id(n) => Id(self.trav_id(n)),
            Const(c) => Const(c),
        }
    }

    fn trav_cond(&mut self, cond: Cond<'ast, Self::InAst>) -> Cond<'ast, Self::OutAst> {
        let c = self.trav_id(cond.cond);
        let t = self.trav_id(cond.then_branch);
        let e = self.trav_id(cond.else_branch);
        Cond { cond: c, then_branch: t, else_branch: e }
    }

    fn trav_call(&mut self, call: Call<'ast, Self::InAst>) -> Self::CallOut {
        let new_args = call.args.into_iter().map(|arg| self.trav_id(arg)).collect();
        Call {
            id: call.id,
            args: new_args,
        }
    }

    fn trav_prf_call(&mut self, prf: PrfCall<'ast, Self::InAst>) -> Self::PrfCallOut {
        use PrfCall::*;
        match prf {
            ShapeA(a) => {
                let a = self.trav_id(a.clone());
                ShapeA(a)
            }
            DimA(a) => {
                let a = self.trav_id(a.clone());
                DimA(a)
            }
            AddSxS(l, r) => {
                let l = self.trav_id(l.clone());
                let r = self.trav_id(r.clone());
                AddSxS(l, r)
            }
            SubSxS(l, r) => {
                let l = self.trav_id(l.clone());
                let r = self.trav_id(r.clone());
                SubSxS(l, r)
            }
            MulSxS(l, r) => {
                let l = self.trav_id(l.clone());
                let r = self.trav_id(r.clone());
                MulSxS(l, r)
            }
            DivSxS(l, r) => {
                let l = self.trav_id(l.clone());
                let r = self.trav_id(r.clone());
                DivSxS(l, r)
            }
            SelVxA(a, b) => {
                let a = self.trav_id(a.clone());
                let b = self.trav_id(b.clone());
                SelVxA(a, b)
            }
            LtSxS(a, b) => {
                let a = self.trav_id(a.clone());
                let b = self.trav_id(b.clone());
                LtSxS(a, b)
            }
            LeSxS(a, b) => {
                let a = self.trav_id(a.clone());
                let b = self.trav_id(b.clone());
                LeSxS(a, b)
            }
            GtSxS(a, b) => {
                let a = self.trav_id(a.clone());
                let b = self.trav_id(b.clone());
                GtSxS(a, b)
            }
            GeSxS(a, b) => {
                let a = self.trav_id(a.clone());
                let b = self.trav_id(b.clone());
                GeSxS(a, b)
            }
            EqSxS(a, b) => {
                let a = self.trav_id(a.clone());
                let b = self.trav_id(b.clone());
                EqSxS(a, b)
            }
            NeSxS(a, b) => {
                let a = self.trav_id(a.clone());
                let b = self.trav_id(b.clone());
                NeSxS(a, b)
            }
            NegS(a) => {
                let a = self.trav_id(a.clone());
                NegS(a)
            }
            NotS(a) => {
                let a = self.trav_id(a.clone());
                NotS(a)
            }
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
        let lb = if let Some(lb) = tensor.lb {
            Some(self.trav_id(lb))
        } else {
            None
        };
        let ub = self.trav_id(tensor.ub);

        let iv_lvis = self.alloc_lvis(tensor.iv.name.clone(), None);
        self.decs.push(iv_lvis);

        self.push_env();
        self.bind_env(tensor.iv.name.clone(), Id::Var(iv_lvis));
        let old_assigns = mem::take(&mut self.new_assigns);

        let mut body = Vec::new();
        for stmt in tensor.body {
            let stmt = self.trav_stmt(stmt);
            body.extend(mem::take(&mut self.new_assigns));
            body.push(stmt);
        }

        let ret = self.trav_id(tensor.ret);
        body.extend(mem::take(&mut self.new_assigns));

        self.new_assigns = old_assigns;
        self.pop_env();

        Tensor {
            body,
            ret,
            iv: iv_lvis,
            lb,
            ub,
        }
    }

    fn trav_fold(&mut self, fold: Fold<'ast, Self::InAst>) -> Self::FoldOut {
        let neutral = self.trav_id(fold.neutral);

        let foldfun = match fold.foldfun {
            FoldFun::Name(id) => FoldFun::Name(id),
            FoldFun::Apply { id, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| match arg {
                        FoldFunArg::Placeholder => FoldFunArg::Placeholder,
                        FoldFunArg::Bound(bound) => FoldFunArg::Bound(self.trav_id(bound)),
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
            values.push(self.trav_id(value));
        }
        Array { elems: values }
    }

    fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Id<'ast, Self::OutAst> {
        match id {
            Id::Arg(i) => Id::Arg(i),
            Id::Var(v) => self
                .lookup_env(&v)
                .unwrap_or_else(|| panic!("could not resolve id {v}")),
        }
    }
}