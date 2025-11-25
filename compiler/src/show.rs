use std::{collections::HashMap, marker::PhantomData};

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
        // Extract var names/types so we can consume the maps while still having lookups
        let mut names: HashMap<VarKey, String> = HashMap::new();
        for (k, v) in fundef.vars.iter() {
            names.insert(k, v.id.clone());
        }

        let get_name = |k: &VarKey| names.get(k).cloned().unwrap_or_else(|| format!("{:?}", k));

        println!("fn {}(", fundef.id);
        let args_list = fundef
            .args
            .iter()
            .map(|k| format!("{} ({:?})", get_name(k), k))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  args: {}", args_list);

        println!("  locals:");
        for (k, name) in &names {
            if fundef.args.contains(k) {
                continue;
            }
            println!("    {:?}: {}", k, name);
        }

        println!("  ssa:");
        for (k, expr) in fundef.ssa.iter() {
            match expr {
                Expr::Binary(Binary { l, r, op }) => {
                    println!("    {} = {} {} {};", get_name(&k), get_name(l), op, get_name(r));
                },
                Expr::Unary(Unary { r, op }) => {
                    println!("    {} = {} {};", get_name(&k), op, get_name(r));
                },
                Expr::Bool(v) => println!("    {} = {};", get_name(&k), v),
                Expr::U32(v) => println!("    {} = {};", get_name(&k), v),
            }
        }

        println!("  return {};", get_name(&fundef.ret_value));
    }
}
