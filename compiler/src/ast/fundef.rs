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
pub enum ScopeEntry<'ast, Ast: AstConfig> {
    Assign {
        avis: &'ast Avis<Ast>,
        expr: &'ast Expr<'ast, Ast>,
    },
    Index {
        avis: &'ast Avis<Ast>,
        lb: ArgOrVar<'ast, Ast>,
        ub: ArgOrVar<'ast, Ast>,
    },
}

impl<'ast, Ast: AstConfig> ScopeEntry<'ast, Ast> {
    pub fn avis(self) -> &'ast Avis<Ast> {
        match self {
            Self::Assign { avis, .. } | Self::Index { avis, .. } => avis,
        }
    }

    pub fn def(self) -> LocalDef<'ast, Ast> {
        match self {
            Self::Assign { expr, .. } => LocalDef::Assign(expr),
            Self::Index { lb, ub, .. } => LocalDef::IndexRange { lb, ub },
        }
    }
}

pub type SsaBlock<'ast, Ast> = Vec<ScopeEntry<'ast, Ast>>;

pub fn find_local_in_scopes<'ast, Ast: AstConfig>(
    scopes: &[SsaBlock<'ast, Ast>],
    key: &'ast Avis<Ast>,
) -> Option<LocalDef<'ast, Ast>> {
    for scope in scopes.iter().rev() {
        for entry in scope.iter().rev() {
            if std::ptr::eq(entry.avis(), key) {
                return Some(entry.def());
            }
        }
    }
    None
}

/// Maybe the whole thing is just overkill, and we should instead just use a refcell tree
/// structure to store the ast, which also allows us to lookup parent nodes.
/// https://github.com/0xSaksham/tree_data_structure
/// The main issue is likely in modifying parent nodes, but
///   we should never modify parent nodes, only ourselves
///   the only exception is maybe to adjust their declaration may (which is scope-local),
///   but maybe that should still be done by the fundef after the fact, by storing changes in the
///   knapsack.
///
///   traversals should then always return a new node (possibly unchanged)
///   to force (eg) fundefs to adjust their map of declarations
///
/// But how to link from this declarations map to the corresponding nodes?
/// Can the reference be used as a key?
#[derive(Clone, Debug)]
pub struct Fundef<'ast, Ast: AstConfig> {
    /// User-defined function name
    pub name: String,
    /// Function arguments
    pub args: Vec<&'ast Avis<Ast>>,
    /// Local identifiers
    pub ids: Vec<&'ast Avis<Ast>>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub ssa: SsaBlock<'ast, Ast>,
    /// Key of the return value
    pub ret: ArgOrVar<'ast, Ast>,
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
        find_local_in_scopes(std::slice::from_ref(&self.ssa), key)
    }
}
