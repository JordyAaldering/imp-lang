use super::{AstConfig, Lvis, ScopeBlock, Stmt, Id};

#[derive(Clone, Debug)]
pub struct Tensor<'ast, Ast: AstConfig> {
    pub body: Vec<Stmt<'ast, Ast>>,
    pub ret: Ast::Operand<'ast>,
    pub iv: &'ast Lvis<'ast, Ast>,
    pub lb: Ast::Operand<'ast>,
    pub ub: Ast::Operand<'ast>,
}

impl<'ast, Ast: AstConfig> Tensor<'ast, Ast> {
    pub fn build_scope(&self) -> ScopeBlock<'ast, Ast>
    where
        Ast::Operand<'ast>: Into<Id<'ast, Ast>>,
    {
        let mut scope = Vec::with_capacity(self.body.len());
        for stmt in &self.body {
            if let Some(entry) = stmt.as_scope_entry() {
                scope.push(entry);
            }
        }
        scope
    }
}
