use std::{io, marker::PhantomData};

use crate::{ast::*, traverse::Visit};

pub struct Show<Ast: AstConfig> {
    w: Box<dyn io::Write>,
    _phantom: PhantomData<Ast>,
}

impl<Ast: AstConfig> Show<Ast> {
    pub fn new(w: Box<dyn io::Write>) -> Self {
        Self {
            w,
            _phantom: PhantomData::default(),
        }
    }

    pub fn show_program(&mut self, program: &Program<Ast>) -> io::Result<()> {
        for fundef in &program.fundefs {
            self.show_fundef(fundef)?;
        }
        Ok(())
    }

    fn show_fundef(&mut self, fundef: &Fundef<Ast>) -> io::Result<()> {
        write!(self.w, "fn {} (", fundef.name)?;
        for avis in &fundef.args {
            write!(self.w, "{:?} {}, ", avis.ty, avis.name)?;
        }
        writeln!(self.w, ") -> {:?} {{", fundef.block.ret)?;

        println!("  vars:");
        for (_, v) in fundef.block.local_vars.iter() {
            println!("    {:?}", v);
        }

        println!("  ssa:");
        for (k, expr) in fundef.block.local_ssa.iter() {
            match expr {
                Expr::Tensor(Tensor { body: expr, iv, lb, ub }) => {
                    println!("    {} = {{ {} | {} <= {} < {} }};", fundef.block.local_vars[k].name, fundef[expr.ret].name, fundef[*lb].name, fundef.block.local_vars[iv.0].name, fundef[*ub].name);
                }
                Expr::Binary(Binary { l, r, op }) => {
                    println!("    {} = {} {} {};", fundef.block.local_vars[k].name, fundef[*l].name, op, fundef[*r].name);
                },
                Expr::Unary(Unary { r, op }) => {
                    println!("    {} = {} {};", fundef.block.local_vars[k].name, op, fundef[*r].name);
                },
                Expr::Bool(v) => println!("    {} = {};", fundef.block.local_vars[k].name, v),
                Expr::U32(v) => println!("    {} = {};", fundef.block.local_vars[k].name, v),
            }
        }

        println!("  return {}", fundef[fundef.block.ret.clone()].name);

        Ok(())
    }
}

impl<Ast: AstConfig> Visit<Program<Ast>> for Show<Ast> {
    type Out = io::Result<()>;

    fn visit(&mut self, program: Program<Ast>) -> Self::Out {
        for fundef in &program.fundefs {
            self.show_fundef(fundef)?;
        }
        Ok(())
    }
}