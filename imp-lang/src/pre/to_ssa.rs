use std::{collections::HashMap, mem};

use typed_arena::Arena;

use crate::ast::*;

pub fn to_ssa<'ast>(program: Program<'ast, FlattenedAst>) -> Program<'ast, UntypedAst> {
        let mut overloads = HashMap::new();
    let fundefs_arena: Arena<Fundef<'ast, UntypedAst>> = Arena::new();

        for (name, groups) in program.overloads {
            let mut new_groups = HashMap::new();

            for (sig, fundefs) in groups {
                let mut new_fundefs = Vec::new();

                for fundef in fundefs {
                    let out_fundef = ToSsa::new().trav_fundef(fundef);
                    let out_ref = fundefs_arena.alloc(out_fundef);
                    let out_ref: &'ast Fundef<'ast, UntypedAst> = unsafe { std::mem::transmute(out_ref) };
                    new_fundefs.push(out_ref);
                }

                new_groups.insert(sig, new_fundefs);
            }

            overloads.insert(name, new_groups);
        }

        Program {
            overloads,
            fundefs: fundefs_arena,
        }
    }

pub struct ToSsa<'ast> {
    uid: usize,
    decs_arena: Arena<VarInfo<'ast, UntypedAst>>,
    expr_arena: Arena<Expr<'ast, UntypedAst>>,
    new_assigns: Vec<Stmt<'ast, UntypedAst>>,
    env_stack: Vec<HashMap<String, Id<'ast, UntypedAst>>>,
}

impl<'ast> ToSsa<'ast> {
    fn new() -> Self {
        Self {
            uid: 0,
            decs_arena: Arena::new(),
            expr_arena: Arena::new(),
            new_assigns: Vec::new(),
            env_stack: Vec::new(),
        }
    }

    fn fresh_uid(&mut self) -> String {
        self.uid += 1;
        format!("_ssa_{}", self.uid)
    }

    fn alloc_lvis(&self, name: String, ssa: Option<&'ast Expr<'ast, UntypedAst>>) -> &'ast VarInfo<'ast, UntypedAst> {
        // SAFETY: The Arena is owned by ToSsa<'ast>, so it lives for 'ast.
        // typed_arena uses interior mutability to allow allocation through &self.
        // The returned reference is valid for 'ast because the Arena will not be dropped
        // until the end of 'ast.
        unsafe {
            std::mem::transmute(self.decs_arena.alloc(VarInfo { name, ty: None, ssa }))
        }
    }

    fn alloc_expr(&self, expr: Expr<'ast, UntypedAst>) -> &'ast Expr<'ast, UntypedAst> {
        // SAFETY: The arena is owned by ToSsa<'ast> and moved into Fundef.
        unsafe { std::mem::transmute(self.expr_arena.alloc(expr)) }
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

    fn trav_fundef(&mut self, fundef: &Fundef<'ast, FlattenedAst>) -> Fundef<'ast, UntypedAst> {
        // Fresh arena for this fundef's declarations
        self.decs_arena = Arena::new();
        self.expr_arena = Arena::new();

        self.push_env();

        let args = self.trav_fargs(fundef.args.clone());

        let mut shape_prelude = Vec::new();
        for assign in &fundef.shape_prelude {
            let assign = self.trav_assign(*assign);
            let new_assigns = mem::take(&mut self.new_assigns);
            for stmt in new_assigns {
                match stmt {
                    Stmt::Assign(n) => shape_prelude.push(n),
                    Stmt::Printf(_) => unreachable!(),
                }
            }
            shape_prelude.push(assign);
        }

        let body = self.trav_body(fundef.body.clone());

        self.pop_env();

        // Move the filled arena to Fundef; no cloning needed
        let decs = mem::take(&mut self.decs_arena);
        let exprs = mem::take(&mut self.expr_arena);

        Fundef {
            name: fundef.name.clone(),
            args,
            shape_prelude,
            shape_facts: fundef.shape_facts.clone(),
            decs,
            exprs,
            body,
            ret_type: fundef.ret_type.clone(),
        }
    }

    fn trav_fargs(&mut self, args: Vec<Farg>) -> Vec<Farg> {
        for (idx, arg) in args.iter().enumerate() {
            self.bind_env(arg.id.clone(), Id::Arg(idx));
        }
        args
    }

    fn trav_body(&mut self, body: Body<'ast, FlattenedAst>) -> Body<'ast, UntypedAst> {
        let old_assigns = mem::take(&mut self.new_assigns);

        let mut stmts = Vec::new();
        for stmt in body.stmts {
            let stmt = self.trav_stmt(stmt);
            stmts.extend(mem::take(&mut self.new_assigns));
            stmts.push(stmt);
        }

        let ret = self.trav_id(body.ret);
        stmts.extend(mem::take(&mut self.new_assigns));

        self.new_assigns = old_assigns;
        Body { stmts, ret }
    }

    fn trav_stmt(&mut self, stmt: Stmt<'ast, FlattenedAst>) -> Stmt<'ast, UntypedAst> {
        use Stmt::*;
        match stmt {
            Assign(n) => Assign(self.trav_assign(n)),
            Printf(n) => Printf(self.trav_printf(n)),
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, FlattenedAst>) -> Assign<'ast, UntypedAst> {
        let old_name = assign.lhs.name.clone();
        let new_name = self.fresh_uid();

        let expr = self.trav_expr((*assign.expr).clone());
        let expr = self.alloc_expr(expr);
        let lvis = self.alloc_lvis(new_name, Some(expr));
        self.bind_env(old_name, Id::Var(lvis));

        Assign { lhs: lvis, expr }
    }

    fn trav_printf(&mut self, printf: Printf<'ast, FlattenedAst>) -> Printf<'ast, UntypedAst> {
        let id = self.trav_id(printf.id);
        Printf { id }
    }

    fn trav_expr(&mut self, expr: Expr<'ast, FlattenedAst>) -> Expr<'ast, UntypedAst> {
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

    fn trav_cond(&mut self, cond: Cond<'ast, FlattenedAst>) -> Cond<'ast, UntypedAst> {
        let c = self.trav_id(cond.cond);
        let t = self.trav_body(cond.then_branch);
        let e = self.trav_body(cond.else_branch);
        Cond { cond: c, then_branch: t, else_branch: e }
    }

    fn trav_call(&mut self, call: Call<'ast, FlattenedAst>) -> Call<'ast, UntypedAst> {
        let new_args = call.args.into_iter().map(|arg| self.trav_id(arg)).collect();
        Call {
            id: call.id,
            args: new_args,
        }
    }

    fn trav_prf_call(&mut self, prf: PrfCall<'ast, FlattenedAst>) -> PrfCall<'ast, UntypedAst> {
        use PrfCall::*;
        match prf {
            DimA(a) => {
                let a = self.trav_id(a.clone());
                DimA(a)
            }
            ShapeA(a) => {
                let a = self.trav_id(a.clone());
                ShapeA(a)
            }
            SelVxA(a, b) => {
                let a = self.trav_id(a.clone());
                let b = self.trav_id(b.clone());
                SelVxA(a, b)
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

    fn trav_tensor(&mut self, tensor: Tensor<'ast, FlattenedAst>) -> Tensor<'ast, UntypedAst> {
        let lb = if let Some(lb) = tensor.lb {
            Some(self.trav_id(lb))
        } else {
            None
        };
        let ub = self.trav_id(tensor.ub);

        let iv_lvis = self.alloc_lvis(tensor.iv.name.clone(), None);

        self.push_env();
        self.bind_env(tensor.iv.name.clone(), Id::Var(iv_lvis));

        let body = self.trav_body(tensor.body);

        self.pop_env();

        Tensor { body, iv: iv_lvis, lb, ub }
    }

    fn trav_fold(&mut self, fold: Fold<'ast, FlattenedAst>) -> Fold<'ast, UntypedAst> {
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

        Fold { neutral, foldfun, selection }
    }

    fn trav_array(&mut self, array: Array<'ast, FlattenedAst>) -> Array<'ast, UntypedAst> {
        let mut values = Vec::with_capacity(array.elems.len());
        for value in array.elems {
            values.push(self.trav_id(value));
        }
        Array { elems: values }
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
