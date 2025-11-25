use std::fmt;

use slotmap::*;

new_key_type! { pub struct VarKey; }
new_key_type! { pub struct ExprKey; }

pub trait AstConfig {
    type ValueType: Clone + fmt::Debug;
}

pub struct UntypedAst;

impl AstConfig for UntypedAst {
    type ValueType = Option<Type>;
}

pub struct TypedAst;

impl AstConfig for TypedAst {
    type ValueType = Type;
}

#[derive(Clone, Debug)]
pub struct VarInfo<Ast: AstConfig> {
    pub key: VarKey,
    pub id: String,
    pub ty: Ast::ValueType,
}

#[derive(Clone, Debug)]
pub struct Program<Ast: AstConfig> {
    pub fundefs: Vec<Fundef<Ast>>,
}

#[derive(Clone, Debug)]
pub struct Fundef<Ast: AstConfig> {
    pub id: String,
    /// ordered 'arena' containing a mapping of argument key to avis info
    ///
    /// Hmm. How can we create a unique varkey if we dont use a slotmap
    /// Perhaps we can use a vec here, turn vars into a secondary map, and then use a default slotmap to contain both
    pub args: Vec<VarKey>,
    //args: Vec<(VarKey, VarInfo)>,
    /// arena containing a mapping of variable keys to avis info
    /// If we look for a key but it does not exist here, it must be an arg
    pub vars: SlotMap<VarKey, VarInfo<Ast>>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub ssa: SecondaryMap<VarKey, Expr>,
    pub ret_value: VarKey,
}

impl<Ast: AstConfig> Fundef<Ast> {
    pub fn insert_var(&mut self, name: &str, ty: Ast::ValueType) -> VarKey {
        let var = VarInfo { key: VarKey::null(), id: name.to_owned(), ty };
        let key = self.vars.insert(var);
        self.vars[key].key = key;
        key
    }
}

#[derive(Clone, Debug)]
pub enum Expr {
    Binary(Binary),
    Unary(Unary),
    // I don't thik var is actually needed. During parsing we do still need such a construct because we lack context
    // (A slotmap does not even exist yet, everything is just identifiers that may or may not exist)
    // But afterwards it is redundant
    //Var(VarKey),
    // We might even be able to do the same thing for constants, if we include this in the type information instead
    // - maybe not actually, as a varkey should come with an ssa as well. But maybe a type is the field of Const does work
    // - or alternatively, a map of constants alongside the ssa map
    Bool(bool),
    U32(u32),
}

#[derive(Clone, Debug)]
pub struct Binary {
    pub l: VarKey,
    pub r: VarKey,
    pub op: Bop,
}

#[derive(Clone, Debug)]
pub struct Unary {
    pub r: VarKey,
    pub op: Uop,
}

#[derive(Copy, Clone, Debug)]
pub enum Bop {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
}

impl fmt::Display for Bop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Bop::*;
        write!(f, "{}", match self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Eq => "==",
            Ne => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Uop {
    Neg, Not,
}

impl fmt::Display for Uop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Uop::*;
        write!(f, "{}", match self {
            Not => "!",
            Neg => "-",
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Type {
    U32,
    Bool,
}
