use super::*;

/// Identifier occurring in an expression position.
#[derive(Clone, Copy, Debug)]
pub enum Id<'ast, Ast: Invariant> {
    /// Formal function argument
    Arg(usize),
    /// Local variable
    Var(Ast::VarLink<'ast>),
}

#[derive(Clone, Debug)]
pub struct VarInfo<'ast, Ast: Invariant> {
    pub name: String,
    pub ty: Ast::VarType,
    pub ssa: Ast::SsaLink<'ast>,
}

impl<'ast, Ast: Invariant> Id<'ast, Ast> {
    // pub fn get(&'ast self, args: &'ast [Ast::VarLink<'ast>]) -> &'ast Ast::VarLink<'ast> {
    //     match self {
    //         Self::Arg(i) => &args[*i],
    //         Self::Var(v) => v,
    //     }
    // }

    pub fn get_name(&self, arg_names: &[String]) -> String {
        match self {
            Self::Arg(i) => arg_names[*i].clone(),
            Self::Var(v) => Ast::var_name(v),
        }
    }
}

impl<'ast> Id<'ast, TypedAst> {
    pub fn get_type<'a>(&'a self, arg_types: &'a [Type]) -> &'a Type {
        match self {
            Self::Arg(i) => &arg_types[*i],
            Self::Var(v) => &v.ty,
        }
    }
}
