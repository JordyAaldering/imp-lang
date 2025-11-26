use crate::ast::*;

pub struct Show;

impl Show {
    pub fn new() -> Self {
        Self
    }

    pub fn show_program(&mut self, program: &Program) {
        for fundef in &program.fundefs {
            self.show_fundef(fundef);
        }
    }

    fn show_fundef(&mut self, fundef: &Fundef) {
        println!("fn {}", fundef.name);

        println!("  args:");
        for avis in &fundef.args {
            println!("    {:?}", avis);
        }

        println!("  locals:");
        for (k, _) in &fundef.ssa {
            println!("    {:?}", fundef[k]);
        }

        println!("  ssa:");
        for (k, expr) in fundef.ssa.iter() {
            match expr {
                Expr::Binary(Binary { l, r, op }) => {
                    println!("    {} = {} {} {};", fundef[k].name, fundef[*l].name, op, fundef[*r].name);
                },
                Expr::Unary(Unary { r, op }) => {
                    println!("    {} = {} {};", fundef.nameof(k), op, fundef[*r].name);
                },
                Expr::Bool(v) => println!("    {} = {};", fundef.nameof(k), v),
                Expr::U32(v) => println!("    {} = {};", fundef.nameof(k), v),
            }
        }

        println!("  return {}", fundef[fundef.ret_id].name);
    }
}
