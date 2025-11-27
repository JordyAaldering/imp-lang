use std::marker::PhantomData;

use crate::ast::*;

pub struct Show<Ast: AstConfig> {
    _phantom: PhantomData<Ast>,
}

impl<Ast: AstConfig> Show<Ast> {
    pub fn new() -> Self {
        Self { _phantom: PhantomData::default() }
    }

    pub fn show_program(&mut self, program: &Program<Ast>) {
        for fundef in &program.fundefs {
            self.show_fundef(fundef);
        }
    }

    fn show_fundef(&mut self, fundef: &Fundef<Ast>) {
        println!("fn {}", fundef.name);

        println!("  args:");
        for avis in &fundef.args {
            println!("    {:?}", avis);
        }

        println!("  locals:");
        for (k, _) in &fundef.ssa {
            println!("    {:?}", fundef.vars[k]);
        }

        println!("  ssa:");
        for (k, expr) in fundef.ssa.iter() {
            match expr {
                Expr::Tensor(Tensor { expr, iv, lb, ub }) => {
                    println!("    {} = {{ {} | {} <= {} < {} }};", fundef.vars[k].name, fundef[expr.clone()].name, fundef[lb.clone()].name, fundef.vars[**iv].name, fundef[ub.clone()].name);
                }
                Expr::Binary(Binary { l, r, op }) => {
                    println!("    {} = {} {} {};", fundef.vars[k].name, fundef[l.clone()].name, op, fundef[r.clone()].name);
                },
                Expr::Unary(Unary { r, op }) => {
                    println!("    {} = {} {};", fundef.vars[k].name, op, fundef[r.clone()].name);
                },
                Expr::Bool(v) => println!("    {} = {};", fundef.vars[k].name, v),
                Expr::U32(v) => println!("    {} = {};", fundef.vars[k].name, v),
            }
        }

        println!("  return {}", fundef[fundef.ret.clone()].name);
    }
}
