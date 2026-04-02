// Declarations
mod program;
mod fundef;
// Statements
mod stmt;
mod assign;
mod ret;
// Expressions
mod expr;
mod tensor;
mod binary;
mod unary;
// Terminals
mod id;
mod avis;
mod typ;

// Declarations
pub use program::*;
pub use fundef::*;
// Statements
pub use stmt::*;
pub use assign::*;
pub use ret::*;
// Expressions
pub use expr::*;
pub use tensor::*;
pub use binary::*;
pub use unary::*;
// Terminals
pub use id::*;
pub use avis::*;
pub use typ::*;

use std::fmt;

use crate::{Traverse, Visit};

pub trait AstConfig: Clone + fmt::Debug {
    type VarType: Clone + fmt::Debug + fmt::Display;

    type VarLink<'ast>: Clone + fmt::Debug;

    type Operand<'ast>: Clone + fmt::Debug;

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String;

    type SsaLink<'ast>: Clone + fmt::Debug;

    fn var_lvis<'ast>(link: &Self::VarLink<'ast>) -> Option<&'ast Lvis<'ast, Self>>;

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: Visit<'ast, Ast = Self> + ?Sized;

    fn trav_operand<'ast, T>(traverser: &mut T, operand: Self::Operand<'ast>) -> T::ExprOut
    where
        T: Traverse<'ast, InAst = Self> + ?Sized,
        T::IdOut: Into<T::ExprOut>;
}

#[derive(Clone, Copy, Debug)]
pub struct ParsedAst;

impl AstConfig for ParsedAst {
    type VarType = MaybeType;

    type VarLink<'ast> = String;

    type Operand<'ast> = &'ast Expr<'ast, ParsedAst>;

    type SsaLink<'ast> = ();

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String {
        link.clone()
    }

    fn var_lvis<'ast>(_link: &Self::VarLink<'ast>) -> Option<&'ast Lvis<'ast, Self>> {
        None
    }

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_expr(*operand);
    }

    fn trav_operand<'ast, T>(traverser: &mut T, operand: Self::Operand<'ast>) -> T::ExprOut
    where
        T: Traverse<'ast, InAst = Self> + ?Sized,
        T::IdOut: Into<T::ExprOut>,
    {
        traverser.trav_expr((*operand).clone()).into()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlattenedAst;

impl AstConfig for FlattenedAst {
    type VarType = MaybeType;

    type VarLink<'ast> = String;

    type Operand<'ast> = Id<'ast, FlattenedAst>;

    type SsaLink<'ast> = ();

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String {
        link.clone()
    }

    fn var_lvis<'ast>(_link: &Self::VarLink<'ast>) -> Option<&'ast Lvis<'ast, Self>> {
        None
    }

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_id(operand);
    }

    fn trav_operand<'ast, T>(traverser: &mut T, operand: Self::Operand<'ast>) -> T::ExprOut
    where
        T: Traverse<'ast, InAst = Self> + ?Sized,
        T::IdOut: Into<T::ExprOut>,
    {
        traverser.trav_id(operand).into()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UntypedAst;

impl AstConfig for UntypedAst {
    type VarType = MaybeType;

    type VarLink<'ast> = &'ast Lvis<'ast, UntypedAst>;

    type Operand<'ast> = Id<'ast, UntypedAst>;

    type SsaLink<'ast> = Option<&'ast Expr<'ast, UntypedAst>>;

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String {
        link.name.clone()
    }

    fn var_lvis<'ast>(link: &Self::VarLink<'ast>) -> Option<&'ast Lvis<'ast, Self>> {
        Some(*link)
    }

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_id(operand);
    }

    fn trav_operand<'ast, T>(traverser: &mut T, operand: Self::Operand<'ast>) -> T::ExprOut
    where
        T: Traverse<'ast, InAst = Self> + ?Sized,
        T::IdOut: Into<T::ExprOut>,
    {
        traverser.trav_id(operand).into()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TypedAst;

impl AstConfig for TypedAst {
    type VarType = Type;

    type VarLink<'ast> = &'ast Lvis<'ast, TypedAst>;

    type Operand<'ast> = Id<'ast, TypedAst>;

    type SsaLink<'ast> = Option<&'ast Expr<'ast, TypedAst>>;

    fn var_name<'ast>(link: &Self::VarLink<'ast>) -> String {
        link.name.clone()
    }

    fn var_lvis<'ast>(link: &Self::VarLink<'ast>) -> Option<&'ast Lvis<'ast, Self>> {
        Some(*link)
    }

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_id(operand);
    }

    fn trav_operand<'ast, T>(traverser: &mut T, operand: Self::Operand<'ast>) -> T::ExprOut
    where
        T: Traverse<'ast, InAst = Self> + ?Sized,
        T::IdOut: Into<T::ExprOut>,
    {
        traverser.trav_id(operand).into()
    }
}
