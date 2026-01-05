use std::io;

use crate::ast::*;

pub struct Show<Ast: AstConfig> {
    w: Box<dyn io::Write>,
    fargs: Vec<Avis<Ast>>,
    scopes: Vec<Block<Ast>>,
}

impl<Ast: AstConfig> Scoped<Ast> for Show<Ast> {
    fn fargs(&self) -> &Vec<Avis<Ast>> {
        &self.fargs
    }

    fn fargs_mut(&mut self) -> &mut Vec<Avis<Ast>> {
        &mut self.fargs
    }

    fn scopes(&self) -> &Vec<Block<Ast>> {
        &self.scopes
    }

    fn scopes_mut(&mut self) -> &mut Vec<Block<Ast>> {
        &mut self.scopes
    }
}

impl<Ast: AstConfig> Show<Ast> {
    pub fn new(w: Box<dyn io::Write>) -> Self {
        Self {
            w,
            fargs: Vec::new(),
            scopes: Vec::new(),
        }
    }

    pub fn show_program(&mut self, program: &Program<Ast>) -> io::Result<()> {
        for fundef in &program.fundefs {
            self.fargs = fundef.args.clone();
            self.scopes.push(fundef.body.clone());

            self.show_fundef(fundef)?;

            self.scopes.pop().unwrap();
            self.fargs = Vec::new();
        }
        Ok(())
    }

    fn show_fundef(&mut self, fundef: &Fundef<Ast>) -> io::Result<()> {
        write!(self.w, "fn {} (", fundef.name)?;
        for avis in &fundef.args {
            write!(self.w, "{:?} {}, ", avis.ty, avis.name)?;
        }
        writeln!(self.w, ") -> {:?} {{", fundef.body.ret)?;

        println!("  vars:");
        for (_, v) in fundef.body.ids.iter() {
            println!("    {:?}", v);
        }

        println!("  ssa:");
        for (k, expr) in fundef.body.ssa.iter() {
            print!("    {} = ", fundef.body.ids[k].name);
            match expr {
                Expr::Tensor(tensor) => {
                    self.show_tensor(tensor)?;
                }
                Expr::Binary(Binary { l, r, op }) => {
                    print!("{} {} {}", fundef[*l].name, op, fundef[*r].name);
                },
                Expr::Unary(Unary { r, op }) => {
                    print!("{} {}", op, fundef[*r].name);
                },
                Expr::Bool(v) => print!("{}", v),
                Expr::U32(v) => print!("{}", v),
            }
            println!(";");
        }

        println!("  return {};", fundef[fundef.body.ret.clone()].name);

        Ok(())
    }

    fn show_tensor(&mut self, tensor: &Tensor<Ast>) -> io::Result<()> {
        let Tensor { body, iv, lb, ub } = tensor;

        self.scopes.push(body.clone());

        println!("{{");
        println!("    local vars:");
        for (_, v) in body.ids.iter() {
            println!("    {:?}", v);
        }

        println!("  local exprs:");
        for (k, expr) in body.ssa.iter() {
            print!("    {} = ", body.ids[k].name);
            match expr {
                Expr::Tensor(tensor) => {
                    self.show_tensor(tensor)?;
                }
                Expr::Binary(Binary { l, r, op }) => {
                    print!("{} {} {}", self.find_id(*l).unwrap().name, op, self.find_id(*r).unwrap().name);
                },
                Expr::Unary(Unary { r, op }) => {
                    print!("{} {}", op, self.find_id(*r).unwrap().name);
                },
                Expr::Bool(v) => print!("{}", v),
                Expr::U32(v) => print!("{}", v),
            }
            println!(";");
        }

        print!("  {} | {} <= {} < {} }}",
            self.find_id(body.ret).unwrap().name,
            self.find_id(*lb).unwrap().name,
            self.find_key(iv.0).unwrap().name,
            self.find_id(*ub).unwrap().name,
        );

        self.scopes.pop().unwrap();
        Ok(())
    }
}
