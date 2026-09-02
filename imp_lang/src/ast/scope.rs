use super::*;

/// Storage backing [`VarInfo`] and [`Expr`] nodes for one [`Invariant`] phase.
///
/// A scope is created once per phase and shared by reference across every pass that builds or extends that phase's AST.
/// The AST never owns it; instead, it operates on references to the [`VarInfo`] and [`ExprCell`] values that live in the arenas.
pub struct Scope<'ast, Ast: Invariant> {
    decs: typed_arena::Arena<VarInfo<'ast, Ast>>,
    exprs: typed_arena::Arena<ExprCell<'ast, Ast>>,
}

impl<'ast, Ast: Invariant> Scope<'ast, Ast> {
    pub fn new() -> Self {
        Self {
            decs: typed_arena::Arena::new(),
            exprs: typed_arena::Arena::new(),
        }
    }

    pub fn alloc_lvis(&'ast self, name: String, ty: Ast::VarType, ssa: Ast::SsaLink<'ast>) -> &'ast VarInfo<'ast, Ast> {
        self.decs.alloc(VarInfo { name, ty, ssa })
    }

    pub fn alloc_expr(&'ast self, expr: Expr<'ast, Ast>) -> &'ast ExprCell<'ast, Ast> {
        self.exprs.alloc(ExprCell::new(expr))
    }
}
