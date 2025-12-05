use std::{collections::HashMap, mem};

use crate::{arena::{Arena, SecondaryArena}, ast::*, scanparse::parse_ast};

pub struct ConvertToSsa {
    uid: usize,
    vars: Arena<Avis<UntypedAst>>,
    ssa: SecondaryArena<Expr>,
    name_to_key: HashMap<String, ArgOrVar>,
}

impl ConvertToSsa {
    pub fn new() -> Self {
        Self {
            uid: 0,
            vars: Arena::new(),
            ssa: SecondaryArena::new(),
            name_to_key: HashMap::new(),
        }
    }

    fn fresh_uid(&mut self) -> String {
        self.uid += 1;
        format!("_ssa_{}", self.uid)
    }

    pub fn convert_program(&mut self, program: parse_ast::Program) -> Program<UntypedAst> {
        let mut fundefs = Vec::new();
        for f in program.fundefs {
            fundefs.push(self.convert_fundef(f));
        }

        Program { fundefs }
    }

    pub fn convert_fundef(&mut self, fundef: parse_ast::Fundef) -> Fundef<UntypedAst> {
        let mut args = Vec::new();
        for (i, (ty, id)) in fundef.args.into_iter().enumerate() {
            args.push(Avis::new(ArgOrVar::Arg(i), &id, Some(ty)));
            self.name_to_key.insert(id, ArgOrVar::Arg(i));
        }

        for stmt in fundef.body {
            self.convert_stmt(stmt);
        }

        let ret_value = self.convert_expr(fundef.ret_expr);
        if let ArgOrVar::Var(k) = ret_value {
            self.vars[k].ty = Some(fundef.ret_type);
        }

        let mut vars = Arena::new();
        mem::swap(&mut self.vars, &mut vars);
        let mut ssa = SecondaryArena::new();
        mem::swap(&mut self.ssa, &mut ssa);
        self.name_to_key.clear();
        self.uid = 0;

        let block = Block {
            local_vars: vars,
            local_ssa: ssa,
            ret: ret_value,
        };

        Fundef {
            name: fundef.id,
            args,
            block,
        }
    }

    pub fn convert_stmt(&mut self, stmt: parse_ast::Stmt) {
        match stmt {
            parse_ast::Stmt::Assign { lhs, expr } => {
                let key = self.convert_expr(expr);
                self.name_to_key.insert(lhs, key);
            },
        }
    }

    pub fn convert_expr(&mut self, expr: parse_ast::Expr) -> ArgOrVar {
        let e = match expr {
            parse_ast::Expr::Tensor { expr, iv, lb, ub } => {
                let key = self.vars.insert_with(|key| {
                    Avis::new(ArgOrVar::Iv(key), &iv.0, None)
                });
                self.name_to_key.insert(iv.0.clone(), ArgOrVar::Iv(key));
                let iv = IndexVector(key);


                let expr = self.convert_expr(*expr);
                let lb = self.convert_expr(*lb);
                let ub = self.convert_expr(*ub);
                Expr::Tensor(Tensor { expr, iv, lb, ub })
            },
            parse_ast::Expr::Binary { l, r, op } => {
                let l_key = self.convert_expr(*l);
                let r_key = self.convert_expr(*r);
                Expr::Binary(Binary { l: l_key, r: r_key, op })
            },
            parse_ast::Expr::Unary { r, op } => {
                let r_key = self.convert_expr(*r);
                Expr::Unary(Unary { r: r_key, op })
            },
            parse_ast::Expr::Bool(v) => {
                Expr::Bool(v)
            }
            parse_ast::Expr::U32(v) => {
                Expr::U32(v)
            },
            parse_ast::Expr::Identifier(id) => {
                return self.name_to_key[&id]
            },
        };

        let id = self.fresh_uid();
        let key = self.vars.insert_with(|key| {
            Avis::new(ArgOrVar::Var(key), &id, None)
        });

        self.ssa.insert(key, e);
        ArgOrVar::Var(key)
    }
}
