use std::collections::HashMap;

use crate::ast::*;

pub fn constant_fold<'ast>(program: &mut Program<'ast, TypedAst>) {
    ConstantFold::new().trav_program(program);
}

pub struct ConstantFold {
    known: HashMap<*const (), u32>,
}

impl ConstantFold {
    pub fn new() -> Self {
        Self {
            known: HashMap::new(),
        }
    }

    fn ptr<'ast>(lvis: &VarInfo<'ast, TypedAst>) -> *const () {
        lvis as *const _ as *const ()
    }

    fn const_u32<'ast>(&self, id: &Id<'ast, TypedAst>) -> Option<u32> {
        match id {
            Id::Var(lvis) => self.known.get(&Self::ptr(lvis)).copied(),
            Id::Arg(_) => None,
        }
    }
}

impl<'ast> Traverse<'ast> for ConstantFold {
    type Ast = TypedAst;

    fn trav_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        self.trav_expr(assign.expr);

        match assign.expr {
            Expr::Const(Const::U32(v)) => {
                self.known.insert(Self::ptr(assign.lhs), *v);
            }
            _ => {
                debug_assert!(!self.known.contains_key(&Self::ptr(assign.lhs)));
            }
        }
    }

    fn trav_prf_expr(&mut self, prf: Prf<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        use Prf::*;
        match &prf {
            AddSxS(l, r) => {
                if let (Some(l), Some(r)) = (self.const_u32(l), self.const_u32(r)) {
                    return Expr::Const(Const::U32(l + r));
                }
            }
            SubSxS(l, r) => {
                if let (Some(l), Some(r)) = (self.const_u32(l), self.const_u32(r)) {
                    return Expr::Const(Const::U32(l - r));
                }
            }
            MulSxS(l, r) => {
                if let (Some(l), Some(r)) = (self.const_u32(l), self.const_u32(r)) {
                    return Expr::Const(Const::U32(l * r));
                }
            }
            DivSxS(l, r) => {
                if let (Some(l), Some(r)) = (self.const_u32(l), self.const_u32(r)) && r != 0 {
                    return Expr::Const(Const::U32(l / r));
                }
            }
            _ => (),
        }

        Expr::Prf(prf)
    }
}
