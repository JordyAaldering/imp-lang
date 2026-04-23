use std::{collections::HashMap, mem};

use typed_arena::Arena;

use crate::ast::*;

pub fn flatten<'ast>(program: Program<'ast, ParsedAst>) -> Program<'ast, ParsedAst> {
    let mut overloads = HashMap::new();
    let fundefs_arena: Arena<Fundef<'ast, ParsedAst>> = Arena::new();

    for (name, groups) in program.overloads {
        let mut new_groups = HashMap::new();

        for (sig, fundefs) in groups {
            let mut new_fundefs = Vec::new();

            for fundef in fundefs {
                let out_fundef = Flatten::new().trav_fundef(fundef);
                let out_ref = fundefs_arena.alloc(out_fundef);
                let out_ref: &'ast Fundef<'ast, ParsedAst> = unsafe { std::mem::transmute(out_ref) };
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

struct Flatten<'ast> {
    uid: usize,
    decs_arena: Arena<VarInfo<'ast, ParsedAst>>,
    expr_arena: Arena<Expr<'ast, ParsedAst>>,
    new_assigns: Vec<Assign<'ast, ParsedAst>>,
    env_stack: Vec<HashMap<String, Id<'ast, ParsedAst>>>,
}

impl<'ast> Flatten<'ast> {
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
        format!("_flat_{}", self.uid)
    }

    fn alloc_lvis(&self, name: String, ty: Option<Type>) -> &'ast VarInfo<'ast, ParsedAst> {
        unsafe { std::mem::transmute(self.decs_arena.alloc(VarInfo { name, ty, ssa: () })) }
    }

    fn alloc_expr(&self, expr: Expr<'ast, ParsedAst>) -> &'ast Expr<'ast, ParsedAst> {
        unsafe { std::mem::transmute(self.expr_arena.alloc(expr)) }
    }

    fn push_env(&mut self) {
        self.env_stack.push(HashMap::new());
    }

    fn pop_env(&mut self) {
        self.env_stack.pop().unwrap();
    }

    fn bind_env(&mut self, name: String, id: Id<'ast, ParsedAst>) {
        self.env_stack.last_mut().expect("missing env").insert(name, id);
    }

    fn lookup_env(&self, name: &str) -> Option<Id<'ast, ParsedAst>> {
        for env in self.env_stack.iter().rev() {
            if let Some(id) = env.get(name) {
                return Some(id.clone());
            }
        }
        None
    }

    fn emit_expr(&mut self, expr: Expr<'ast, ParsedAst>) -> &'ast Expr<'ast, ParsedAst> {
        let name = self.fresh_uid();
        let lvis = self.alloc_lvis(name.clone(), None);
        let rhs = self.alloc_expr(expr);
        self.new_assigns.push(Assign { lhs: lvis, expr: rhs });

        let id = Id::Var(name.clone());
        self.bind_env(name, id.clone());
        self.alloc_expr(Expr::Id(id))
    }

    fn trav_fundef(&mut self, fundef: &Fundef<'ast, ParsedAst>) -> Fundef<'ast, ParsedAst> {
        self.decs_arena = Arena::new();
        self.expr_arena = Arena::new();

        self.push_env();

        for (i, arg) in fundef.args.iter().enumerate() {
            self.bind_env(arg.id.clone(), Id::Arg(i));
        }

        let mut shape_prelude = Vec::new();
        for assign in &fundef.shape_prelude {
            let assign = self.trav_assign(*assign);
            let new_assigns = mem::take(&mut self.new_assigns);
            shape_prelude.extend(new_assigns);
            shape_prelude.push(assign);
        }

        let body = self.trav_body(fundef.body.clone());

        self.pop_env();

        let decs = mem::take(&mut self.decs_arena);
        let exprs = mem::take(&mut self.expr_arena);

        Fundef {
            name: fundef.name.clone(),
            args: fundef.args.clone(),
            shape_prelude,
            shape_facts: fundef.shape_facts.clone(),
            decs,
            exprs,
            body,
            ret_type: fundef.ret_type.clone(),
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, ParsedAst>) -> Assign<'ast, ParsedAst> {
        let rhs = self.trav_expr((*assign.expr).clone());
        let lhs_name = assign.lhs.name.clone();
        let lhs_lvis = self.alloc_lvis(lhs_name.clone(), assign.lhs.ty.clone());

        self.bind_env(lhs_name.clone(), Id::Var(lhs_name));

        Assign {
            lhs: lhs_lvis,
            expr: rhs,
        }
    }

    fn trav_printf(&mut self, printf: Printf<'ast, ParsedAst>) -> Printf<'ast, ParsedAst> {
        let id = self.trav_id(printf.id);
        Printf { id }
    }

    fn trav_body(&mut self, body: Body<'ast, ParsedAst>) -> Body<'ast, ParsedAst> {
        let old_assigns = mem::take(&mut self.new_assigns);

        let mut stmts = Vec::new();

        for stmt in body.stmts {
            let stmt = self.trav_stmt(stmt);
            for new_assign in mem::take(&mut self.new_assigns) {
                stmts.push(Stmt::Assign(new_assign));
            }
            stmts.push(stmt);
        }

        let ret = self.trav_expr((*body.ret).clone());
        for new_assign in mem::take(&mut self.new_assigns) {
            stmts.push(Stmt::Assign(new_assign));
        }

        self.new_assigns = old_assigns;
        Body { stmts, ret }
    }

    fn trav_stmt(&mut self, stmt: Stmt<'ast, ParsedAst>) -> Stmt<'ast, ParsedAst> {
        use Stmt::*;
        match stmt {
            Assign(n) => Assign(self.trav_assign(n)),
            Printf(n) => Printf(self.trav_printf(n)),
        }
    }

    fn trav_expr(&mut self, expr: Expr<'ast, ParsedAst>) -> &'ast Expr<'ast, ParsedAst> {
        use Expr::*;
        let expr = match expr {
            Id(n) => {
                let id = self.trav_id(n);
                return self.alloc_expr(Expr::Id(id));
            }
            Cond(n) => Cond(self.trav_cond(n)),
            Call(n) => Call(self.trav_call(n)),
            Prf(n) => Prf(self.trav_prf_call(n)),
            Fold(n) => Fold(self.trav_fold(n)),
            Tensor(n) => Tensor(self.trav_tensor(n)),
            Array(n) => Array(self.trav_array(n)),
            Const(c) => Const(c),
        };
        self.emit_expr(expr)
    }

    fn trav_cond(&mut self, cond: Cond<'ast, ParsedAst>) -> Cond<'ast, ParsedAst> {
        let c = self.trav_expr((*cond.cond).clone());
        let t = self.trav_body(cond.then_branch);
        let e = self.trav_body(cond.else_branch);
        Cond {
            cond: c,
            then_branch: t,
            else_branch: e,
        }
    }

    fn trav_call(&mut self, call: Call<'ast, ParsedAst>) -> Call<'ast, ParsedAst> {
        let mut args = Vec::with_capacity(call.args.len());
        for arg in call.args {
            args.push(self.trav_expr((*arg).clone()));
        }

        Call { id: call.id, args }
    }

    fn trav_prf_call(&mut self, prf: Prf<'ast, ParsedAst>) -> Prf<'ast, ParsedAst> {
        use Prf::*;
        match prf {
            ShapeA(a) => ShapeA(self.trav_expr((*a).clone())),
            DimA(a) => DimA(self.trav_expr((*a).clone())),
            AddSxS(l, r) => AddSxS(self.trav_expr((*l).clone()), self.trav_expr((*r).clone())),
            SubSxS(l, r) => SubSxS(self.trav_expr((*l).clone()), self.trav_expr((*r).clone())),
            MulSxS(l, r) => MulSxS(self.trav_expr((*l).clone()), self.trav_expr((*r).clone())),
            DivSxS(l, r) => DivSxS(self.trav_expr((*l).clone()), self.trav_expr((*r).clone())),
            SelVxA(a, b) => SelVxA(self.trav_expr((*a).clone()), self.trav_expr((*b).clone())),
            LtSxS(a, b) => LtSxS(self.trav_expr((*a).clone()), self.trav_expr((*b).clone())),
            LeSxS(a, b) => LeSxS(self.trav_expr((*a).clone()), self.trav_expr((*b).clone())),
            GtSxS(a, b) => GtSxS(self.trav_expr((*a).clone()), self.trav_expr((*b).clone())),
            GeSxS(a, b) => GeSxS(self.trav_expr((*a).clone()), self.trav_expr((*b).clone())),
            EqSxS(a, b) => EqSxS(self.trav_expr((*a).clone()), self.trav_expr((*b).clone())),
            NeSxS(a, b) => NeSxS(self.trav_expr((*a).clone()), self.trav_expr((*b).clone())),
            NegS(a) => NegS(self.trav_expr((*a).clone())),
            NotS(a) => NotS(self.trav_expr((*a).clone())),
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, ParsedAst>) -> Tensor<'ast, ParsedAst> {
        let lb = tensor.lb.map(|lb| self.trav_expr((*lb).clone()));
        let ub = self.trav_expr((*tensor.ub).clone());

        self.push_env();
        self.bind_env(tensor.iv.name.clone(), Id::Var(tensor.iv.name.clone()));

        let body = self.trav_body(tensor.body);

        self.pop_env();

        let iv = self.alloc_lvis(tensor.iv.name.clone(), tensor.iv.ty.clone());

        Tensor { body, iv, lb, ub }
    }

    fn trav_fold(&mut self, fold: Fold<'ast, ParsedAst>) -> Fold<'ast, ParsedAst> {
        let neutral = self.trav_expr((*fold.neutral).clone());

        let foldfun = match fold.foldfun {
            FoldFun::Name(id) => FoldFun::Name(id),
            FoldFun::Apply { id, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| match arg {
                        FoldFunArg::Placeholder => FoldFunArg::Placeholder,
                        FoldFunArg::Bound(bound) => FoldFunArg::Bound(self.trav_expr((*bound).clone())),
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

    fn trav_array(&mut self, array: Array<'ast, ParsedAst>) -> Array<'ast, ParsedAst> {
        let mut values = Vec::with_capacity(array.elems.len());
        for value in array.elems {
            values.push(self.trav_expr((*value).clone()));
        }
        Array { elems: values }
    }

    fn trav_id(&mut self, id: Id<'ast, ParsedAst>) -> Id<'ast, ParsedAst> {
        match id {
            Id::Arg(i) => Id::Arg(i),
            Id::Var(name) => self.lookup_env(&name).unwrap_or(Id::Var(name)),
        }
    }
}
