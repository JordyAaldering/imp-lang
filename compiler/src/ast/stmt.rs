use super::{Id, Assign, AstConfig, Avis, Return};

pub type ScopeBlock<'ast, Ast> = Vec<ScopeEntry<'ast, Ast>>;

pub type ScopeStack<'ast, Ast> = Vec<ScopeBlock<'ast, Ast>>;

#[derive(Clone, Copy, Debug)]
pub enum Stmt<'ast, Ast: AstConfig> {
    Assign(Assign<'ast, Ast>),
    Return(Return<'ast, Ast>),
}

#[derive(Clone, Copy, Debug)]
pub enum ScopeEntry<'ast, Ast: AstConfig> {
    Assign {
        avis: &'ast Avis<Ast>,
        expr: &'ast super::Expr<'ast, Ast>,
    },
    IndexRange {
        iv: &'ast Avis<Ast>,
        lb: Id<'ast, Ast>,
        ub: Id<'ast, Ast>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum LocalDef<'ast, Ast: AstConfig> {
    Assign(&'ast super::Expr<'ast, Ast>),
    IndexRange {
        lb: Id<'ast, Ast>,
        ub: Id<'ast, Ast>,
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
            Self::Assign { avis, .. } | Self::IndexRange { iv: avis, .. } => avis,
        }
    }

    pub fn def(self) -> Option<LocalDef<'ast, Ast>> {
        match self {
            Self::Assign { expr, .. } => Some(LocalDef::Assign(expr)),
            Self::IndexRange { lb, ub, .. } => Some(LocalDef::IndexRange { lb, ub }),
        }
    }
}
