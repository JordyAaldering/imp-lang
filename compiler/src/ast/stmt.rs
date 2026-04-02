use super::*;

pub type ScopeBlock<'ast, Ast> = Vec<ScopeEntry<'ast, Ast>>;

pub type ScopeStack<'ast, Ast> = Vec<ScopeBlock<'ast, Ast>>;

#[derive(Clone, Debug)]
pub enum Stmt<'ast, Ast: AstConfig> {
    Assign(Assign<'ast, Ast>),
    Return(Return<'ast, Ast>),
}

#[derive(Clone, Debug)]
pub enum ScopeEntry<'ast, Ast: AstConfig> {
    Assign {
        lvis: &'ast VarInfo<'ast, Ast>,
        expr: &'ast super::Expr<'ast, Ast>,
    },
    IndexRange {
        iv: &'ast VarInfo<'ast, Ast>,
        lb: Id<'ast, Ast>,
        ub: Id<'ast, Ast>,
    },
}

#[derive(Clone, Debug)]
pub enum LocalDef<'ast, Ast: AstConfig> {
    Assign(&'ast super::Expr<'ast, Ast>),
    IndexRange {
        lb: Id<'ast, Ast>,
        ub: Id<'ast, Ast>,
    },
}

impl<'ast, Ast: AstConfig> Stmt<'ast, Ast> {
    pub fn as_scope_entry(&self) -> Option<ScopeEntry<'ast, Ast>> {
        match self {
            Self::Assign(Assign { lvis, expr }) => Some(ScopeEntry::Assign { lvis: *lvis, expr: *expr }),
            Self::Return(_) => None,
        }
    }
}

impl<'ast, Ast: AstConfig> ScopeEntry<'ast, Ast> {
    pub fn lvis(&self) -> &'ast VarInfo<'ast, Ast> {
        match self {
            Self::Assign { lvis, .. } | Self::IndexRange { iv: lvis, .. } => lvis,
        }
    }

    pub fn def(&self) -> Option<LocalDef<'ast, Ast>> {
        match self {
            Self::Assign { expr, .. } => Some(LocalDef::Assign(*expr)),
            Self::IndexRange { lb, ub, .. } => Some(LocalDef::IndexRange {
                lb: lb.clone(),
                ub: ub.clone(),
            }),
        }
    }
}
