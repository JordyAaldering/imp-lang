use super::AstConfig;

#[derive(Clone, Debug)]
pub struct Avis<'ast, Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::ValueType,
    pub _marker: std::marker::PhantomData<&'ast Ast>,
}

#[derive(Clone, Copy, Debug)]
pub enum ArgOrVar<'ast, Ast: AstConfig> {
    /// Function argument
    Arg(usize),
    /// Local variable
    Var(&'ast Avis<'ast, Ast>),
    /// Index vector
    Iv(&'ast Avis<'ast, Ast>),
}
