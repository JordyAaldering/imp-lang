use super::*;

#[derive(Clone, Debug)]
pub struct Fundef<'ast, Ast: AstConfig> {
    pub name: String,
    pub ret_type: Type,
    pub args: Vec<Farg>,
    pub shape_prelude: Vec<Assign<'ast, Ast>>,
    pub shape_facts: ShapeFacts,
    pub decs: Vec<&'ast VarInfo<'ast, Ast>>,
    pub body: Vec<Stmt<'ast, Ast>>,
}

#[derive(Clone, Debug)]
pub struct Farg {
    pub id: String,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BaseSignature {
    pub base_types: Vec<BaseType>,
}

impl<'ast, Ast: AstConfig> Fundef<'ast, Ast> {
    pub fn signature(&self) -> BaseSignature {
        BaseSignature {
            base_types: self.args.iter().map(|arg| arg.ty.ty.clone()).collect(),
        }
    }
}
