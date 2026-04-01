use super::{ArgOrVar, Assign, AstConfig, Avis, Return};

#[derive(Clone, Copy, Debug)]
pub enum Stmt<'ast, Ast: AstConfig> {
    Assign(Assign<'ast, Ast>),
    Return(Return<'ast, Ast>),
}

/// Internal scope entries used for definition lookup during lowering and codegen.
///
/// Unlike `Stmt`, this includes index range bindings that do not represent
/// user-level statements.
#[derive(Clone, Copy, Debug)]
pub enum ScopeEntry<'ast, Ast: AstConfig> {
    Assign {
        avis: &'ast Avis<Ast>,
        expr: &'ast super::Expr<'ast, Ast>,
    },
    IndexRange {
        avis: &'ast Avis<Ast>,
        lb: ArgOrVar<'ast, Ast>,
        ub: ArgOrVar<'ast, Ast>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum LocalDef<'ast, Ast: AstConfig> {
    Assign(&'ast super::Expr<'ast, Ast>),
    IndexRange {
        lb: ArgOrVar<'ast, Ast>,
        ub: ArgOrVar<'ast, Ast>,
    },
}

impl<'ast, Ast: AstConfig> Stmt<'ast, Ast> {
    pub fn as_scope_entry(self) -> Option<ScopeEntry<'ast, Ast>> {
        match self {
            Self::Assign(Assign { avis, expr }) => Some(ScopeEntry::Assign { avis, expr }),
            Self::Return(_) => None,
        }
    }
}

impl<'ast, Ast: AstConfig> ScopeEntry<'ast, Ast> {
    pub fn avis(self) -> &'ast Avis<Ast> {
        match self {
            Self::Assign { avis, .. } | Self::IndexRange { avis, .. } => avis,
        }
    }

    pub fn def(self) -> Option<LocalDef<'ast, Ast>> {
        match self {
            Self::Assign { expr, .. } => Some(LocalDef::Assign(expr)),
            Self::IndexRange { lb, ub, .. } => Some(LocalDef::IndexRange { lb, ub }),
        }
    }
}
