use super::*;

#[derive(Clone, Debug)]
pub struct Fundef<'ast, Ast: AstConfig> {
    pub name: String,
    pub ret_type: Type,
    pub args: Vec<Farg>,
    pub decs: Vec<&'ast VarInfo<'ast, Ast>>,
    pub body: Vec<Stmt<'ast, Ast>>,
}

#[derive(Clone, Debug)]
pub struct Farg {
    pub id: String,
    pub ty: Type,
}
