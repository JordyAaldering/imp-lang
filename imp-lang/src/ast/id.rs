use super::*;

/// Identifier occurring in an expression position.
#[derive(Clone, Copy, Debug)]
pub enum Id<'ast, Ast: AstConfig> {
    /// Formal function argument
    Arg(usize),
    Var(Ast::VarLink<'ast>),
    /// The rank (`arr.dim`) of the argument at the given index — bound by a `d:shp` pattern
    Dim(usize),
    /// The shape pointer of the argument at the given index — bound by a `d:shp` pattern
    Shp(usize),
    /// The size of the `dim_idx`-th dimension of the argument at `arg_idx` — bound by a `DimPattern::Var`
    DimAt(usize, usize),
}

#[derive(Clone, Debug)]
pub struct VarInfo<'ast, Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::VarType,
    pub ssa: Ast::SsaLink<'ast>,
}

