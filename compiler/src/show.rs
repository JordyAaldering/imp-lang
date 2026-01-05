use std::io;

use crate::{arena::Key, ast::*, traverse::Scoped};

pub struct Show<Ast: AstConfig> {
    w: Box<dyn io::Write>,
    args: Vec<Avis<Ast>>,
    scopes: Vec<Block<Ast>>,
}

impl<Ast: AstConfig> Scoped<Ast> for Show<Ast> {
    fn find_id(&self, key: Key) -> Option<&Avis<Ast>> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.ids.get(key) {
                return  Some(v);
            }
        }

        None
    }

    fn find_ssa(&self, key: Key) -> Option<&Expr<Ast>> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.ssa.get(key) {
                return  Some(v);
            }
        }

        None
    }

    fn push_scope(&mut self, scope: Block<Ast>) {
        self.scopes.push(scope);
    }

    fn pop_scope(&mut self) -> Block<Ast> {
        self.scopes.pop().unwrap()
    }
}

impl<Ast: AstConfig> Show<Ast> {
    pub fn new(w: Box<dyn io::Write>) -> Self {
        Self {
            w,
            args: Vec::new(),
            scopes: Vec::new(),
        }
    }

    pub fn find_id_or_arg(&mut self, key: ArgOrVar) -> Option<&Avis<Ast>> {
        match key {
            ArgOrVar::Arg(i) => self.args.get(i),
            ArgOrVar::Var(key) => self.find_id(key),
            ArgOrVar::Iv(key) => self.find_id(key),
        }
    }

    pub fn show_program(&mut self, program: &Program<Ast>) -> io::Result<()> {
        for fundef in &program.fundefs {
            self.args = fundef.args.clone();
            self.scopes.push(fundef.block.clone());

            self.show_fundef(fundef)?;

            self.scopes.pop().unwrap();
            self.args = Vec::new();
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
        for (_, v) in fundef.block.ids.iter() {
            println!("    {:?}", v);
        }

        println!("  ssa:");
        for (k, expr) in fundef.block.ssa.iter() {
            print!("    {} = ", fundef.block.ids[k].name);
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

        println!("  return {};", fundef[fundef.block.ret.clone()].name);

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
                    print!("binarytodo");
                },
                Expr::Unary(Unary { r, op }) => {
                    print!("unarytodo");
                },
                Expr::Bool(v) => print!("{}", v),
                Expr::U32(v) => print!("{}", v),
            }
            println!(";");
        }

        print!("  {} | {} <= {} < {} }}",
            self.find_id_or_arg(body.ret).unwrap().name.clone(),
            self.find_id_or_arg(*lb).unwrap().name.clone(),
            self.find_id(iv.0).unwrap().name.clone(),
            self.find_id_or_arg(*ub).unwrap().name.clone(),
        );

        self.scopes.pop().unwrap();
        Ok(())
    }
}
