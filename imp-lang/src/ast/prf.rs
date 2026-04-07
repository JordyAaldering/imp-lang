use super::*;

/// Primitive function call
#[derive(Clone, Debug)]
pub struct PrfCall<'ast, Ast: AstConfig> {
    pub id: Prf,
    pub args: Vec<Ast::Operand<'ast>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Prf {
    /// @selVxA
    ///
    /// Selection of a vector in an array, where the
    /// length of the vector must match the array's rank.
    /// I.e., the result is a scalar.
    ///
    /// `A[V]`
    SelVxA,
    /// @addSxS
    ///
    /// `S + S`
    AddSxS,
    /// @subSxS
    ///
    /// `S - S`
    SubSxS,
    /// @mulSxS
    ///
    /// `S * S`
    MulSxS,
    /// @divSxS
    ///
    /// `S / S`
    DivSxS,
    /// @ltSxS
    ///
    /// `S < S`
    LtSxS,
    /// @leSxS
    ///
    /// `S <= S`
    LeSxS,
    /// @gtSxS
    ///
    /// `S > S`
    GtSxS,
    /// @geSxS
    ///
    /// `S >= S`
    GeSxS,
    /// @eqSxS
    ///
    /// `S == S`
    EqSxS,
    /// @neSxS
    ///
    /// `S != S`
    NeSxS,
    /// @negS
    ///
    /// Unary negation
    ///
    /// `-S`
    NegS,
    /// @notS
    ///
    /// Logical negation
    ///
    /// `!S`
    NotS,
}

impl fmt::Display for Prf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Prf::*;
        match self {
            SelVxA => write!(f, "@selVxA"),
            AddSxS => write!(f, "@addSxS"),
            SubSxS => write!(f, "@subSxS"),
            MulSxS => write!(f, "@mulSxS"),
            DivSxS => write!(f, "@divSxS"),
            LtSxS => write!(f, "@ltSxS"),
            LeSxS => write!(f, "@leSxS"),
            GtSxS => write!(f, "@gtSxS"),
            GeSxS => write!(f, "@geSxS"),
            EqSxS => write!(f, "@eqSxS"),
            NeSxS => write!(f, "@neSxS"),
            NegS => write!(f, "@negS"),
            NotS => write!(f, "@notS"),
        }
    }
}

impl TryFrom<&str> for Prf {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use Prf::*;
        match value {
            "selVxA" => Ok(SelVxA),
            "addSxS" => Ok(AddSxS),
            "subSxS" => Ok(SubSxS),
            "mulSxS" => Ok(MulSxS),
            "divSxS" => Ok(DivSxS),
            "ltSxS" => Ok(LtSxS),
            "leSxS" => Ok(LeSxS),
            "gtSxS" => Ok(GtSxS),
            "geSxS" => Ok(GeSxS),
            "eqSxS" => Ok(EqSxS),
            "neSxS" => Ok(NeSxS),
            "negS" => Ok(NegS),
            "notS" => Ok(NotS),
            _ => Err(()),
        }
    }
}
