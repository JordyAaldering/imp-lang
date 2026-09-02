use super::*;

/// Owns the bump-allocated storage backing `VarInfo`/`Expr` nodes for one `AstConfig` phase.
///
/// A single `Arenas` value is created once (by the compilation driver) per phase and shared,
/// by reference, across every pass that builds or extends that phase's AST (e.g. the parser,
/// then `flatten`/`analyse_tp`, all operating on `ParsedAst`). Because the arena is never owned
/// by the AST values built from it, `alloc_lvis`/`alloc_expr` hand back references that are
/// genuinely valid for `'ast` -- no lifetime-extending `unsafe` is required.
pub struct Scope<'ast, Ast: AstConfig> {
    decs: typed_arena::Arena<VarInfo<'ast, Ast>>,
    exprs: typed_arena::Arena<ExprCell<'ast, Ast>>,
}

impl<'ast, Ast: AstConfig> Scope<'ast, Ast> {
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
