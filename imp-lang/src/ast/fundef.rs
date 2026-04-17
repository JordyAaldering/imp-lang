use super::*;
use typed_arena::Arena;

pub struct Fundef<'ast, Ast: AstConfig> {
    pub name: String,
    pub ret_type: Type,
    pub args: Vec<Farg>,
    pub shape_prelude: Vec<Assign<'ast, Ast>>,
    pub shape_facts: ShapeFacts,
    pub decs: Arena<VarInfo<'ast, Ast>>,
    pub body: Body<'ast, Ast>,
}

impl<'ast, Ast: AstConfig> Clone for Fundef<'ast, Ast> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            ret_type: self.ret_type.clone(),
            args: self.args.clone(),
            shape_prelude: self.shape_prelude.clone(),
            shape_facts: self.shape_facts.clone(),
            decs: Arena::new(),
            body: self.body.clone(),
        }
    }
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
