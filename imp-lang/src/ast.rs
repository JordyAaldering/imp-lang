// Declarations
mod program;
mod fundef;
mod shapefact;
// Statements
mod body;
mod stmt;
mod assign;
mod printf;
// Expressions
mod expr;
mod cond;
mod calltarget;
mod call;
mod prf;
mod tensor;
mod fold;
mod array;
// Terminals
mod id;
mod constval;
mod typ;

// Declarations
pub use program::*;
pub use fundef::*;
pub use shapefact::*;
// Statements
pub use body::*;
pub use stmt::*;
pub use assign::*;
pub use printf::*;
// Expressions
pub use expr::*;
pub use cond::*;
pub use calltarget::*;
pub use call::*;
pub use prf::*;
pub use tensor::*;
pub use fold::*;
pub use array::*;
// Terminals
pub use id::*;
pub use constval::*;
pub use typ::*;

use std::fmt;

use crate::Traverse;

pub trait AstConfig: Clone + fmt::Debug {
    type VarType: Clone + fmt::Debug;

    type VarLink<'ast>: Clone + fmt::Debug;

    type SsaLink<'ast>: Clone + fmt::Debug;

    type Dispatch<'ast>: Clone + fmt::Debug;

    type Operand<'ast>: Clone + fmt::Debug;

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String;

    fn dispatch_name<'ast>(dispatch: &Self::Dispatch<'ast>) -> String;

    fn trav_type<'ast, V>(trav: &mut V, ty: &mut Self::VarType)
    where
        V: Traverse<'ast, Ast = Self> + ?Sized;

    fn trav_operand<'ast, V>(trav: &mut V, operand: &mut Self::Operand<'ast>)
    where
        V: Traverse<'ast, Ast = Self> + ?Sized;
}

#[derive(Clone, Copy, Debug)]
pub struct ParsedAst;

impl AstConfig for ParsedAst {
    type VarType = Option<Type>;

    type VarLink<'ast> = String;

    type SsaLink<'ast> = ();

    type Dispatch<'ast> = String;

    type Operand<'ast> = &'ast Expr<'ast, ParsedAst>;

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String {
        link.clone()
    }

    fn dispatch_name<'ast>(dispatch: &Self::Dispatch<'ast>) -> String {
        dispatch.clone()
    }

    fn trav_type<'ast, V>(trav: &mut V, ty: &mut Self::VarType)
    where
        V: Traverse<'ast, Ast = Self> + ?Sized
    {
        if let Some(ty) = ty {
            trav.trav_type(ty);
        }
    }

    fn trav_operand<'ast, V>(trav: &mut V, operand: &mut Self::Operand<'ast>)
    where
        V: Traverse<'ast, Ast = Self> + ?Sized,
    {
        trav.trav_expr(operand);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UntypedAst;

impl AstConfig for UntypedAst {
    type VarType = Option<Type>;

    type VarLink<'ast> = &'ast VarInfo<'ast, UntypedAst>;

    type SsaLink<'ast> = Option<&'ast Expr<'ast, UntypedAst>>;

    type Dispatch<'ast> = String;

    type Operand<'ast> = Id<'ast, UntypedAst>;

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String {
        link.name.clone()
    }

    fn dispatch_name<'ast>(dispatch: &Self::Dispatch<'ast>) -> String {
        dispatch.clone()
    }

    fn trav_type<'ast, V>(trav: &mut V, ty: &mut Self::VarType)
    where
        V: Traverse<'ast, Ast = Self> + ?Sized
    {
        if let Some(ty) = ty {
            trav.trav_type(ty);
        }
    }

    fn trav_operand<'ast, V>(trav: &mut V, operand: &mut Self::Operand<'ast>)
    where
        V: Traverse<'ast, Ast = Self> + ?Sized,
    {
        trav.trav_id(operand);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TypedAst;

impl AstConfig for TypedAst {
    type VarType = Type;

    type VarLink<'ast> = &'ast VarInfo<'ast, TypedAst>;

    type SsaLink<'ast> = Option<&'ast Expr<'ast, TypedAst>>;

    type Dispatch<'ast> = CallTarget<'ast, TypedAst>;

    type Operand<'ast> = Id<'ast, TypedAst>;

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String {
        link.name.clone()
    }

    fn dispatch_name<'ast>(dispatch: &Self::Dispatch<'ast>) -> String {
        dispatch.name()
    }

    fn trav_type<'ast, V>(trav: &mut V, ty: &mut Self::VarType)
    where
        V: Traverse<'ast, Ast = Self> + ?Sized
    {
        trav.trav_type(ty);
    }

    fn trav_operand<'ast, V>(trav: &mut V, operand: &mut Self::Operand<'ast>)
    where
        V: Traverse<'ast, Ast = Self> + ?Sized,
    {
        trav.trav_id(operand);
    }
}
