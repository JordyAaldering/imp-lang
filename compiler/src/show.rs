use std::io;

use crate::{arena::{Arena, SecondaryArena}, ast::*};

pub struct Show<Ast: AstConfig> {
    w: Box<dyn io::Write>,
    fargs: Vec<Avis<Ast>>,
    scopes: Vec<(Arena<Avis<Ast>>, SecondaryArena<Expr<Ast>>)>,
}

impl<Ast: AstConfig> Scoped<Ast> for Show<Ast> {
    fn fargs(&self) -> &Vec<Avis<Ast>> {
        &self.fargs
    }

    fn fargs_mut(&mut self) -> &mut Vec<Avis<Ast>> {
        &mut self.fargs
    }

    fn scopes(&self) -> &Vec<(Arena<Avis<Ast>>, SecondaryArena<Expr<Ast>>)> {
        &self.scopes
    }

    fn push_scope(&mut self, ids: Arena<Avis<Ast>>, ssa: SecondaryArena<Expr<Ast>>) {
        self.scopes.push((ids, ssa));
    }

    fn pop_scope(&mut self) -> (Arena<Avis<Ast>>, SecondaryArena<Expr<Ast>>) {
        self.scopes.pop().unwrap()
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
            self.scopes.push((fundef.ids.clone(), fundef.ssa.clone()));

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
        writeln!(self.w, ") -> {:?} {{", fundef.ret)?;

        println!("  vars:");
        for (_, v) in fundef.ids.iter() {
            println!("    {:?}", v);
        }

        println!("  ssa:");
        for (k, expr) in fundef.ssa.iter() {
            print!("    {} = ", fundef.ids[k].name);
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

        println!("  return {};", fundef[fundef.ret.clone()].name);

        Ok(())
    }

    fn show_tensor(&mut self, tensor: &Tensor<Ast>) -> io::Result<()> {
        self.scopes.push((tensor.ids.clone(), tensor.ssa.clone()));

        println!("{{");
        println!("    local vars:");
        for (_, v) in tensor.ids.iter() {
            println!("    {:?}", v);
        }

        println!("  local exprs:");
        for (k, expr) in tensor.ssa.iter() {
            print!("    {} = ", tensor.ids[k].name);
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
            self.find_id(tensor.ret).unwrap().name,
            self.find_id(tensor.lb).unwrap().name,
            self.find_key(tensor.iv.0).unwrap().name,
            self.find_id(tensor.ub).unwrap().name,
        );

        self.scopes.pop().unwrap();
        Ok(())
    }
}
