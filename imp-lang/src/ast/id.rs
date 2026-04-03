use super::*;

#[derive(Clone, Copy, Debug)]
pub enum Id<'ast, Ast: AstConfig> {
    Arg(usize),
    Var(Ast::VarLink<'ast>),
    /// The rank (`arr.dim`) of the argument at the given index — bound by a `d:shp` pattern.
    Dim(usize),
    /// The shape pointer of the argument at the given index — bound by a `d:shp` pattern.
    Shp(usize),
    /// The size of the `dim_idx`-th dimension of the argument at `arg_idx` — bound by a `DimPattern::Var`.
    DimAt(usize, usize),
}

#[derive(Clone, Debug)]
pub struct VarInfo<'ast, Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::VarType,
    pub ssa: Ast::SsaLink<'ast>,
}

impl<'ast, Ast: AstConfig> Id<'ast, Ast> {
    pub fn as_local(&self) -> Option<&Ast::VarLink<'ast>> {
        match self {
            Self::Arg(_) | Self::Dim(_) | Self::Shp(_) | Self::DimAt(_, _) => None,
            Self::Var(link) => Some(link),
        }
    }
}
