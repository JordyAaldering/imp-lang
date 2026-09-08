use std::mem;

use crate::{ast::*, trav_name::TravName};

pub fn flatten<'ast>(program: &mut Program<'ast, ParsedAst>, scope: &'ast Scope<'ast, ParsedAst>) {
    Flatten::new(scope).trav_program(program);
}

struct Flatten<'ast> {
    scope: &'ast Scope<'ast, ParsedAst>,
    new_decs: Vec<&'ast VarInfo<'ast, ParsedAst>>,
    new_assigns: Vec<Assign<'ast, ParsedAst>>,
    uid: TravName,
}

impl<'ast> Flatten<'ast> {
    fn new(scope: &'ast Scope<'ast, ParsedAst>) -> Self {
        Self {
            scope,
            new_decs: Vec::new(),
            new_assigns: Vec::new(),
            uid: TravName::new(crate::Phase::FLT),
        }
    }

    fn emit_var(&mut self, name: String, ty: Option<Type>) -> &'ast VarInfo<'ast, ParsedAst> {
        let var = self.scope.alloc_avis(name, ty, ());
        self.new_decs.push(var);
        var
    }

    fn emit_expr(&mut self, expr: Expr<'ast, ParsedAst>) -> Expr<'ast, ParsedAst> {
        let name = self.uid.next();
        let lhs = self.emit_var(name.clone(), None);
        let expr = self.scope.alloc_expr(expr);
        self.new_assigns.push(Assign { lhs, expr });
        Expr::Id(Id::Var(name))
    }
}

impl<'ast> Traverse<'ast> for Flatten<'ast> {
    type Ast = ParsedAst;

    type DeclOut = ();

    type ExprOut = ();

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, ParsedAst>) {
        debug_assert!(self.new_decs.is_empty());
        debug_assert!(self.new_assigns.is_empty());

        let mut shape_prelude = Vec::new();
        for mut assign in fundef.shape_prelude.drain(..) {
            self.trav_assign(&mut assign);
            shape_prelude.extend(mem::take(&mut self.new_assigns));
            shape_prelude.push(assign);
        }
        fundef.shape_prelude = shape_prelude;

        self.trav_body(&mut fundef.body);

        fundef.decs.extend(mem::take(&mut self.new_decs));
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
        if let Expr::Id(_) = expr {
            (expr, ())
        } else {
            let expr = expr.visit(self).0;
            let id = self.emit_expr(expr);
            (id, ())
        }
    }
}
