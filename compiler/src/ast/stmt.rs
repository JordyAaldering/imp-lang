use super::{ArgOrVar, AstConfig, Avis, Expr};

#[derive(Clone, Copy, Debug)]
pub enum Stmt<'ast, Ast: AstConfig> {
    Assign {
        avis: &'ast Avis<Ast>,
        expr: &'ast Expr<'ast, Ast>,
    },
    Index {
        avis: &'ast Avis<Ast>,
        lb: ArgOrVar<'ast, Ast>,
        ub: ArgOrVar<'ast, Ast>,
    },
    Return {
        id: ArgOrVar<'ast, Ast>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum LocalDef<'ast, Ast: AstConfig> {
    Assign(&'ast Expr<'ast, Ast>),
    IndexRange {
        lb: ArgOrVar<'ast, Ast>,
        ub: ArgOrVar<'ast, Ast>,
    },
}

impl<'ast, Ast: AstConfig> Stmt<'ast, Ast> {
    pub fn avis(self) -> Option<&'ast Avis<Ast>> {
        match self {
            Self::Assign { avis, .. } | Self::Index { avis, .. } => Some(avis),
            Self::Return { .. } => None,
        }
    }

    pub fn def(self) -> Option<LocalDef<'ast, Ast>> {
        match self {
            Self::Assign { expr, .. } => Some(LocalDef::Assign(expr)),
            Self::Index { lb, ub, .. } => Some(LocalDef::IndexRange { lb, ub }),
            Self::Return { .. } => None,
        }
    }
}
