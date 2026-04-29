use std::mem;

use typed_arena::Arena;

use crate::{ast::*, trav_name::TravName};

pub fn flatten<'ast>(program: &mut Program<'ast, ParsedAst>) {
    Flatten::new().trav_program(program);
}

struct Flatten<'ast> {
    trav_name: TravName,
    decs: Arena<VarInfo<'ast, ParsedAst>>,
    exprs: Arena<Expr<'ast, ParsedAst>>,
    new_assigns: Vec<Assign<'ast, ParsedAst>>,
}

impl<'ast> Flatten<'ast> {
    fn new() -> Self {
        Self {
            trav_name: TravName::new(crate::Phase::FLT),
            decs: Arena::new(),
            exprs: Arena::new(),
            new_assigns: Vec::new(),
        }
    }

    fn alloc_lvis(&self, name: String, ty: Option<Type>) -> &'ast VarInfo<'ast, ParsedAst> {
        unsafe { std::mem::transmute(self.decs.alloc(VarInfo { name, ty, ssa: () })) }
    }

    fn alloc_expr(&self, expr: Expr<'ast, ParsedAst>) -> &'ast Expr<'ast, ParsedAst> {
        unsafe { std::mem::transmute(self.exprs.alloc(expr)) }
    }

    fn emit_expr(&mut self, expr: Expr<'ast, ParsedAst>) -> Expr<'ast, ParsedAst> {
        let name = self.trav_name.next();
        let lvis = self.alloc_lvis(name.clone(), None);
        let rhs = self.alloc_expr(expr);
        self.new_assigns.push(Assign { lhs: lvis, expr: rhs });
        Expr::Id(Id::Var(name))
    }
}

impl<'ast> Traverse<'ast> for Flatten<'ast> {
    type Ast = ParsedAst;

    type ExprOut = ();

    const EXPR_DEFAULT: Self::ExprOut = ();

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, ParsedAst>) {
        debug_assert!(self.decs.len() == 0);
        debug_assert!(self.exprs.len() == 0);
        debug_assert!(self.new_assigns.is_empty());

        self.decs = mem::take(&mut fundef.decs);
        self.exprs = mem::take(&mut fundef.exprs);

        let mut shape_prelude = Vec::new();
        for mut assign in fundef.shape_prelude.drain(..) {
            self.trav_assign(&mut assign);
            shape_prelude.extend(mem::take(&mut self.new_assigns));
            shape_prelude.push(assign);
        }
        fundef.shape_prelude = shape_prelude;

        self.trav_body(&mut fundef.body);

        fundef.decs = mem::take(&mut self.decs);
        fundef.exprs = mem::take(&mut self.exprs);
    }

    fn trav_body(&mut self, body: &mut Body<'ast, ParsedAst>) {
        let old_assigns = mem::take(&mut self.new_assigns);

        let mut stmts = Vec::new();
        for mut stmt in body.stmts.drain(..) {
            self.trav_stmt(&mut stmt);
            stmts.extend(mem::take(&mut self.new_assigns).into_iter().map(Stmt::Assign));
            stmts.push(stmt);
        }

        self.trav_expr(&mut body.ret);
        stmts.extend(mem::take(&mut self.new_assigns).into_iter().map(Stmt::Assign));

        body.stmts = stmts;

        self.new_assigns = old_assigns;
    }

    fn trav_expr_value(&mut self, expr: Expr<'ast, Self::Ast>) -> (Expr<'ast, Self::Ast>, Self::ExprOut) {
        use Expr::*;
        let (expr, _) = match expr {
            Id(n) => {
                return (Id(n), Self::EXPR_DEFAULT);
            }
            Cond(n) => self.trav_cond_expr(n),
            Call(n) => self.trav_call_expr(n),
            Prf(n) => self.trav_prf_expr(n),
            Tensor(n) => self.trav_tensor_expr(n),
            Fold(n) => self.trav_fold_expr(n),
            Array(n) => self.trav_array_expr(n),
            Const(n) => self.trav_const_expr(n),
        };

        let id = self.emit_expr(expr);
        (id, Self::EXPR_DEFAULT)
    }
}
