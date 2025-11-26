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
        println!("fn {}(", fundef.id);
        let args_list = fundef
            .args
            .iter()
            .map(|k| format!("{} ({:?})", fundef.nameof(*k), k))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  args: {}", args_list);

        println!("  locals:");
        for argkey in &fundef.args {
            println!("    {:?}: {}", argkey, fundef.nameof(*argkey));
        }

        println!("  ssa:");
        for (k, expr) in fundef.ssa.iter() {
            match expr {
                Expr::Binary(Binary { l, r, op }) => {
                    println!("    {} = {} {} {};", fundef.nameof(k), fundef.nameof(*l), op, fundef.nameof(*r));
                },
                Expr::Unary(Unary { r, op }) => {
                    println!("    {} = {} {};", fundef.nameof(k), op, fundef.nameof(*r));
                },
                Expr::Bool(v) => println!("    {} = {};", fundef.nameof(k), v),
                Expr::U32(v) => println!("    {} = {};", fundef.nameof(k), v),
            }
        }

        println!("  return {};", fundef.nameof(fundef.ret_value));
    }
}
