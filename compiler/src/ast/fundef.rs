use super::{ArgOrVar, AstConfig, Avis, Expr};

#[derive(Clone, Copy, Debug)]
pub enum LocalDef<'ast, Ast: AstConfig> {
    Assign(&'ast Expr<'ast, Ast>),
    IndexRange {
        lb: ArgOrVar<'ast, Ast>,
        ub: ArgOrVar<'ast, Ast>,
    },
}

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

pub type SsaBlock<'ast, Ast> = Vec<Stmt<'ast, Ast>>;

pub fn find_local_in_scopes<'ast, Ast: AstConfig>(
    scopes: &[SsaBlock<'ast, Ast>],
    key: &'ast Avis<Ast>,
) -> Option<LocalDef<'ast, Ast>> {
    for scope in scopes.iter().rev() {
        for stmt in scope.iter().rev() {
            let Some(avis) = stmt.avis() else {
                continue;
            };

            if std::ptr::eq(avis, key) {
                return stmt.def();
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct Fundef<'ast, Ast: AstConfig> {
    pub name: String,
    pub args: Vec<&'ast Avis<Ast>>,
    pub ids: Vec<&'ast Avis<Ast>>,
    pub body: SsaBlock<'ast, Ast>,
}

impl<'ast, Ast: AstConfig> Fundef<'ast, Ast> {
    pub fn avis_of(&self, key: ArgOrVar<'ast, Ast>) -> &'ast Avis<Ast> {
        match key {
            ArgOrVar::Arg(i) => self.args[i],
            ArgOrVar::Var(v) => v,
        }
    }

    pub fn nameof(&self, k: ArgOrVar<'ast, Ast>) -> &str {
        &self.avis_of(k).name
    }

    pub fn typof(&self, k: ArgOrVar<'ast, Ast>) -> &Ast::ValueType {
        &self.avis_of(k).ty
    }

    pub fn ret_id(&self) -> ArgOrVar<'ast, Ast> {
        for stmt in self.body.iter().rev() {
            if let Stmt::Return { id } = *stmt {
                return id;
            }
        }

        panic!("fundef body must end in a return statement")
    }

    pub fn arg_index(&self, key: ArgOrVar<'ast, Ast>) -> Option<usize> {
        match key {
            ArgOrVar::Arg(i) => Some(i),
            ArgOrVar::Var(_) => None,
        }
    }

    pub fn find_ssa(&self, key: &'ast Avis<Ast>) -> Option<&'ast Expr<'ast, Ast>> {
        self.find_local_def(key).and_then(|def| {
            if let LocalDef::Assign(expr) = def {
                Some(expr)
            } else {
                None
            }
        })
    }

    pub fn find_local_def(&self, key: &'ast Avis<Ast>) -> Option<LocalDef<'ast, Ast>> {
        find_local_in_scopes(std::slice::from_ref(&self.body), key)
    }
}
