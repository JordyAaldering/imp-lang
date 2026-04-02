use crate::ast::Type;

use super::{Id, AstConfig, Lvis, LocalDef, ScopeBlock, Stmt};

#[derive(Clone, Debug)]
pub struct Fundef<'ast, Ast: AstConfig> {
    pub name: String,
    pub ret_type: Type,
    pub args: Vec<&'ast Farg>,
    pub decs: Vec<&'ast Lvis<'ast, Ast>>,
    pub body: Vec<Stmt<'ast, Ast>>,
}

#[derive(Clone, Debug)]
pub struct Farg {
    pub name: String,
    pub ty: Type,
}

impl<'ast, Ast: AstConfig> Fundef<'ast, Ast> {
    pub fn nameof(&self, k: &Id<'ast, Ast>) -> String {
        match k {
            Id::Arg(i) => self.args[*i].name.clone(),
            Id::Var(v) => Ast::var_name(v),
        }
    }

    pub fn ret_id(&self) -> Id<'ast, Ast> {
        for stmt in self.body.iter().rev() {
            if let Stmt::Return(ret) = stmt {
                return ret.id.clone();
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
            .filter_map(|stmt| stmt.as_scope_entry())
            .collect()
    }

    pub fn find_local_def(&self, key: &'ast Lvis<'ast, Ast>) -> Option<LocalDef<'ast, Ast>> {
        let body_scope = self.scope_block();
        find_local_in_scopes(&vec![body_scope], key)
    }
}

pub fn find_local_in_scopes<'ast, Ast: AstConfig>(scopes: &Vec<ScopeBlock<'ast, Ast>>, key: &'ast Lvis<'ast, Ast>) -> Option<LocalDef<'ast, Ast>> {
    for scope in scopes.iter().rev() {
        for stmt in scope.iter().rev() {
            let lvis = stmt.lvis();
            if std::ptr::eq(lvis, key) {
                return stmt.def();
            }
        }
    }
    None
}
