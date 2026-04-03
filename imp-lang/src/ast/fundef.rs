use super::*;

#[derive(Clone, Debug)]
pub struct Fundef<'ast, Ast: AstConfig> {
    pub name: String,
    pub ret_type: Type,
    pub args: Vec<&'ast Farg>,
    pub decs: Vec<&'ast VarInfo<'ast, Ast>>,
    pub body: Vec<Stmt<'ast, Ast>>,
}

#[derive(Clone, Debug)]
pub struct Farg {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct GenericFundef<'ast, Ast: AstConfig> {
    pub name: String,
    pub type_param: String,
    pub where_bounds: Vec<TraitBound>,
    pub ret_type: PolyType,
    pub args: Vec<PolyArg>,
    pub decs: Vec<&'ast VarInfo<'ast, Ast>>,
    pub body: Vec<Stmt<'ast, Ast>>,
}
