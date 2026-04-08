use super::*;

/// Primitive function call
#[derive(Clone, Debug)]
pub enum PrfCall<'ast, Ast: AstConfig> {
    /// @selVxA
    ///
    /// Selection of a vector in an array, where the
    /// length of the vector must match the array's rank.
    /// I.e., the result is a scalar.
    ///
    /// `A[V]`
    SelVxA(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @addSxS
    ///
    /// `S + S`
    AddSxS(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @subSxS
    ///
    /// `S - S`
    SubSxS(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @mulSxS
    ///
    /// `S * S`
    MulSxS(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @divSxS
    ///
    /// `S / S`
    DivSxS(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @ltSxS
    ///
    /// `S < S`
    LtSxS(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @leSxS
    ///
    /// `S <= S`
    LeSxS(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @gtSxS
    ///
    /// `S > S`
    GtSxS(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @geSxS
    ///
    /// `S >= S`
    GeSxS(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @eqSxS
    ///
    /// `S == S`
    EqSxS(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @neSxS
    ///
    /// `S != S`
    NeSxS(Ast::Operand<'ast>, Ast::Operand<'ast>),
    /// @negS
    ///
    /// Unary negation
    ///
    /// `-S`
    NegS(Ast::Operand<'ast>),
    /// @notS
    ///
    /// Logical negation
    ///
    /// `!S`
    NotS(Ast::Operand<'ast>),
}

impl<'ast, Ast: AstConfig> PrfCall<'ast, Ast> {
    pub fn nameof(&self) -> &'static str {
        use PrfCall::*;
        match self {
            SelVxA(_, _) => "@selVxA",
            AddSxS(_, _) => "@addSxS",
            SubSxS(_, _) => "@subSxS",
            MulSxS(_, _) => "@mulSxS",
            DivSxS(_, _) => "@divSxS",
            LtSxS(_, _) => "@ltSxS",
            LeSxS(_, _) => "@leSxS",
            GtSxS(_, _) => "@gtSxS",
            GeSxS(_, _) => "@geSxS",
            EqSxS(_, _) => "@eqSxS",
            NeSxS(_, _) => "@neSxS",
            NegS(_) => "@negS",
            NotS(_) => "@notS",
        }
    }

    pub fn args(&self) -> Vec<&Ast::Operand<'ast>> {
        use PrfCall::*;
        match self {
            SelVxA(a, b) => vec![a, b],
            AddSxS(a, b) => vec![a, b],
            SubSxS(a, b) => vec![a, b],
            MulSxS(a, b) => vec![a, b],
            DivSxS(a, b) => vec![a, b],
            LtSxS(a, b) => vec![a, b],
            LeSxS(a, b) => vec![a, b],
            GtSxS(a, b) => vec![a, b],
            GeSxS(a, b) => vec![a, b],
            EqSxS(a, b) => vec![a, b],
            NeSxS(a, b) => vec![a, b],
            NegS(a) => vec![a],
            NotS(a) => vec![a],
        }
    }

    pub fn args_mut(&mut self) -> Vec<&mut Ast::Operand<'ast>> {
        use PrfCall::*;
        match self {
            SelVxA(a, b) => vec![a, b],
            AddSxS(a, b) => vec![a, b],
            SubSxS(a, b) => vec![a, b],
            MulSxS(a, b) => vec![a, b],
            DivSxS(a, b) => vec![a, b],
            LtSxS(a, b) => vec![a, b],
            LeSxS(a, b) => vec![a, b],
            GtSxS(a, b) => vec![a, b],
            GeSxS(a, b) => vec![a, b],
            EqSxS(a, b) => vec![a, b],
            NeSxS(a, b) => vec![a, b],
            NegS(a) => vec![a],
            NotS(a) => vec![a],
        }
    }
}
