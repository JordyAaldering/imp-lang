// Declarations
mod program;
mod fundef;
mod shapefact;
// Statements
mod stmt;
mod assign;
mod ret;
// Expressions
mod expr;
mod cond;
mod calltarget;
mod call;
mod prf;
mod fold;
mod tensor;
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
pub use stmt::*;
pub use assign::*;
pub use ret::*;
// Expressions
pub use expr::*;
pub use cond::*;
pub use calltarget::*;
pub use call::*;
pub use prf::*;
pub use fold::*;
pub use tensor::*;
pub use array::*;
// Terminals
pub use id::*;
pub use constval::*;
pub use typ::*;

use std::fmt;

use crate::Visit;

pub trait AstConfig: Clone + fmt::Debug {
    type VarType: Clone + fmt::Debug;

    type VarLink<'ast>: Clone + fmt::Debug;

    type SsaLink<'ast>: Clone + fmt::Debug;

    type Dispatch<'ast>: Clone + fmt::Debug;

    type Operand<'ast>: Clone + fmt::Debug;

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String;

    fn visit_type<'ast, V>(visitor: &mut V, ty: &Self::VarType)
    where
        V: Visit<'ast, Ast = Self> + ?Sized;

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: Visit<'ast, Ast = Self> + ?Sized;

    /// Extract the function name from a dispatch value, for display and codegen.
    fn dispatch_name<'ast>(dispatch: &Self::Dispatch<'ast>) -> String;
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

    fn visit_type<'ast, V>(visitor: &mut V, ty: &Self::VarType)
    where
        V: Visit<'ast, Ast = Self> + ?Sized
    {
        if let Some(ty) = ty {
            visitor.visit_type(ty);
        }
    }

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_expr(*operand);
    }

    fn dispatch_name<'ast>(dispatch: &Self::Dispatch<'ast>) -> String {
        dispatch.clone()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlattenedAst;

impl AstConfig for FlattenedAst {
    type VarType = Option<Type>;

    type VarLink<'ast> = String;

    type SsaLink<'ast> = ();

    type Dispatch<'ast> = String;

    type Operand<'ast> = Id<'ast, FlattenedAst>;

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String {
        link.clone()
    }

    fn visit_type<'ast, V>(visitor: &mut V, ty: &Self::VarType)
    where
        V: Visit<'ast, Ast = Self> + ?Sized
    {
        if let Some(ty) = ty {
            visitor.visit_type(ty);
        }
    }

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_id(operand);
    }

    fn dispatch_name<'ast>(dispatch: &Self::Dispatch<'ast>) -> String {
        dispatch.clone()
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

    fn visit_type<'ast, V>(visitor: &mut V, ty: &Self::VarType)
    where
        V: Visit<'ast, Ast = Self> + ?Sized
    {
        if let Some(ty) = ty {
            visitor.visit_type(ty);
        }
    }

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_id(operand);
    }

    fn dispatch_name<'ast>(dispatch: &Self::Dispatch<'ast>) -> String {
        dispatch.clone()
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

    fn visit_type<'ast, V>(visitor: &mut V, ty: &Self::VarType)
    where
        V: Visit<'ast, Ast = Self> + ?Sized
    {
        visitor.visit_type(ty);
    }

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_id(operand);
    }

    fn dispatch_name<'ast>(dispatch: &Self::Dispatch<'ast>) -> String {
        dispatch.name()
    }
}
