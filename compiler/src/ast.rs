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

pub trait AstConfig: Clone + Copy + fmt::Debug {
    type ValueType: Clone + fmt::Debug + fmt::Display;
    type Operand<'ast>: Clone + Copy + fmt::Debug;

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: crate::traverse::Visit<'ast, Ast = Self> + ?Sized;

    fn trav_operand<'ast, T>(traverser: &mut T, operand: Self::Operand<'ast>) -> T::ExprOut
    where
        T: crate::traverse::Traverse<'ast, InAst = Self> + ?Sized,
        T::IdOut: Into<T::ExprOut>;
}

#[derive(Clone, Copy, Debug)]
pub struct UntypedAst;

impl AstConfig for UntypedAst {
    type ValueType = MaybeType;
    type Operand<'ast> = Id<'ast, UntypedAst>;

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: crate::traverse::Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_id(operand);
    }

    fn trav_operand<'ast, T>(traverser: &mut T, operand: Self::Operand<'ast>) -> T::ExprOut
    where
        T: crate::traverse::Traverse<'ast, InAst = Self> + ?Sized,
        T::IdOut: Into<T::ExprOut>,
    {
        traverser.trav_id(operand).into()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TypedAst;

impl AstConfig for TypedAst {
    type ValueType = Type;
    type Operand<'ast> = Id<'ast, TypedAst>;

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: crate::traverse::Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_id(operand);
    }

    fn trav_operand<'ast, T>(traverser: &mut T, operand: Self::Operand<'ast>) -> T::ExprOut
    where
        T: crate::traverse::Traverse<'ast, InAst = Self> + ?Sized,
        T::IdOut: Into<T::ExprOut>,
    {
        traverser.trav_id(operand).into()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UnflattenedAst;

impl AstConfig for UnflattenedAst {
    type ValueType = MaybeType;
    type Operand<'ast> = &'ast Expr<'ast, UnflattenedAst>;

    fn visit_operand<'ast, V>(visitor: &mut V, operand: &Self::Operand<'ast>)
    where
        V: crate::traverse::Visit<'ast, Ast = Self> + ?Sized,
    {
        visitor.visit_expr(*operand);
    }

    fn trav_operand<'ast, T>(traverser: &mut T, operand: Self::Operand<'ast>) -> T::ExprOut
    where
        T: crate::traverse::Traverse<'ast, InAst = Self> + ?Sized,
        T::IdOut: Into<T::ExprOut>,
    {
        traverser.trav_expr((*operand).clone()).into()
    }
}
