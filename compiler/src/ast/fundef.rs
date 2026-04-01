use super::{Id, AstConfig, Avis, LocalDef, ScopeBlock, Stmt};

#[derive(Clone, Debug)]
pub struct Fundef<'ast, Ast: AstConfig> {
    pub name: String,
    pub args: Vec<&'ast Avis<Ast>>,
    pub decs: Vec<&'ast Avis<Ast>>,
    pub body: Vec<Stmt<'ast, Ast>>,
}

impl<'ast, Ast: AstConfig> Fundef<'ast, Ast> {
    fn avis_of(&self, key: Id<'ast, Ast>) -> &'ast Avis<Ast> {
        match key {
            Id::Arg(i) => self.args[i],
            Id::Var(v) => v,
        }
    }

    pub fn nameof(&self, k: Id<'ast, Ast>) -> &str {
        &self.avis_of(k).name
    }

    pub fn typof(&self, k: Id<'ast, Ast>) -> &Ast::ValueType {
        &self.avis_of(k).ty
    }

    pub fn ret_id(&self) -> Id<'ast, Ast> {
        for stmt in self.body.iter().rev() {
            if let Stmt::Return(ret) = *stmt {
                return ret.id;
            }
        }

        panic!("fundef body must end in a return statement")
    }

    pub fn arg_index(&self, key: Id<'ast, Ast>) -> Option<usize> {
        match key {
            Id::Arg(i) => Some(i),
            Id::Var(_) => None,
        }
    }

    pub fn scope_block(&self) -> ScopeBlock<'ast, Ast> {
        self.body
            .iter()
            .filter_map(|stmt| (*stmt).as_scope_entry())
            .collect()
    }

    pub fn find_local_def(&self, key: &'ast Avis<Ast>) -> Option<LocalDef<'ast, Ast>> {
        let body_scope = self.scope_block();
        find_local_in_scopes(std::slice::from_ref(&body_scope), key)
    }
}

pub fn find_local_in_scopes<'ast, Ast: AstConfig>(scopes: &[ScopeBlock<'ast, Ast>], key: &'ast Avis<Ast>) -> Option<LocalDef<'ast, Ast>> {
    for scope in scopes.iter().rev() {
        for stmt in scope.iter().rev() {
            let avis = stmt.avis();
            if std::ptr::eq(avis, key) {
                return stmt.def();
            }
        }
    }
    None
}
