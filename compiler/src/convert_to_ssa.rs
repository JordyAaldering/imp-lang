use std::{collections::HashMap, mem};

use crate::{arena::{Arena, SecondaryArena}, ast::*, scanparse::parse_ast};

pub struct ConvertToSsa {
    uid: usize,
    vars: Arena<Avis<UntypedAst>>,
    ssa: SecondaryArena<Expr>,
    name_to_key: HashMap<String, ArgOrVar>,
}

#[derive(Debug)]
pub enum SsaError {
    MissingReturnStatement,
}

type SsaResult<T> = Result<T, SsaError>;

impl ConvertToSsa {
    pub fn new() -> Self {
        Self {
            uid: 0,
            vars: Arena::new(),
            ssa: SecondaryArena::new(),
            name_to_key: HashMap::new(),
        }
    }

    fn fresh_uid(&mut self, id: Option<&str>) -> String {
        // TODO: generate a name that the user could never write, e.g. something containing `@`
        // This requires us to sanitize these strings again before compiling
        let s = format!("_{}_{}", id.unwrap_or("ssa"), self.uid);
        self.uid += 1;
        s
    }

    pub fn convert_program(&mut self, program: parse_ast::Program) -> SsaResult<Program<UntypedAst>> {
        let mut fundefs = Vec::new();
        for f in program.fundefs {
            fundefs.push(self.convert_fundef(f)?);
        }

        Ok(Program { fundefs })
    }

    pub fn convert_fundef(&mut self, fundef: parse_ast::Fundef) -> SsaResult<Fundef<UntypedAst>> {
        // Reset self
        self.uid = 0;
        self.name_to_key.clear();

        let mut args = Vec::new();
        for (i, (ty, id)) in fundef.args.into_iter().enumerate() {
            args.push(Avis::new(ArgOrVar::Arg(i), &id, Some(ty)));
            self.name_to_key.insert(id, ArgOrVar::Arg(i));
        }

        let ret_value = self.convert_body(fundef.body)?;
        if let ArgOrVar::Var(k) = ret_value {
            self.vars[k].ty = Some(fundef.ret_type);
        }

        let mut vars = Arena::new();
        mem::swap(&mut self.vars, &mut vars);
        let mut ssa = SecondaryArena::new();
        mem::swap(&mut self.ssa, &mut ssa);

        Ok(Fundef {
            name: fundef.id,
            args,
            vars,
            ssa,
            ret: ret_value,
        })
    }

    pub fn convert_body(&mut self, body: Vec<parse_ast::Stmt>) -> SsaResult<ArgOrVar> {
        for stmt in body {
            if let Some(ret_value_key) = self.convert_stmt(stmt)? {
                // A return statement was encountered, we can stop now
                // Note that this no longer works when we add branching
                return Ok(ret_value_key);
            }

            // Otherwise, keep converting
        }

        // We converted all statements without finding a return
        Err(SsaError::MissingReturnStatement)
    }

    pub fn convert_stmt(&mut self, stmt: parse_ast::Stmt) -> SsaResult<Option<ArgOrVar>> {
        match stmt {
            parse_ast::Stmt::Assign { lhs, expr } => {
                // We need explicit handling of the outermost expression, which is why we don't call convert_expr immediately
                // We can probably make this nicer though
                let e = match expr {
                    parse_ast::Expr::Tensor { expr, iv, lb, ub } => {
                        let key = self.vars.insert_with(|key| {
                            Avis::new(ArgOrVar::Iv(key), &iv.0, None)
                        });
                        self.name_to_key.insert(iv.0.clone(), ArgOrVar::Iv(key));
                        let iv = IndexVector(key);

                        let expr = self.convert_expr(*expr)?;
                        let lb = self.convert_expr(*lb)?;
                        let ub = self.convert_expr(*ub)?;
                        Expr::Tensor(Tensor { expr, iv, lb, ub })
                    },
                    parse_ast::Expr::Binary { l, r, op } => {
                        let l_key = self.convert_expr(*l)?;
                        let r_key = self.convert_expr(*r)?;
                        Expr::Binary(Binary { l: l_key, r: r_key, op })
                    },
                    parse_ast::Expr::Unary { r, op } => {
                        let r_key = self.convert_expr(*r)?;
                        Expr::Unary(Unary { r: r_key, op })
                    },
                    parse_ast::Expr::Identifier(id) => {
                        println!("Searching for {}", id);
                        return Ok(Some(self.name_to_key[&id].clone()));
                    },
                    parse_ast::Expr::Bool(v) => {
                        Expr::Bool(v)
                    }
                    parse_ast::Expr::U32(v) => {
                        Expr::U32(v)
                    },
                };

                let id = self.fresh_uid(Some(&lhs));
                let key = self.vars.insert_with(|key| {
                    Avis::new(ArgOrVar::Var(key), &id, None)
                });

                self.ssa.insert(key, e);

                self.name_to_key.insert(lhs, ArgOrVar::Var(key));

                Ok(None)
            },
            parse_ast::Stmt::Return { expr } => {
                let ret_value_key = self.convert_expr(expr)?;
                Ok(Some(ret_value_key))
            },
        }
    }

    pub fn convert_expr(&mut self, expr: parse_ast::Expr) -> SsaResult<ArgOrVar> {
        let e = match expr {
            parse_ast::Expr::Tensor { expr, iv, lb, ub } => {
                let key = self.vars.insert_with(|key| {
                    Avis::new(ArgOrVar::Iv(key), &iv.0, None)
                });
                self.name_to_key.insert(iv.0.clone(), ArgOrVar::Iv(key));
                let iv = IndexVector(key);

                let expr = self.convert_expr(*expr)?;
                let lb = self.convert_expr(*lb)?;
                let ub = self.convert_expr(*ub)?;
                Expr::Tensor(Tensor { expr, iv, lb, ub })
            },
            parse_ast::Expr::Binary { l, r, op } => {
                let l_key = self.convert_expr(*l)?;
                let r_key = self.convert_expr(*r)?;
                Expr::Binary(Binary { l: l_key, r: r_key, op })
            },
            parse_ast::Expr::Unary { r, op } => {
                let r_key = self.convert_expr(*r)?;
                Expr::Unary(Unary { r: r_key, op })
            },
            parse_ast::Expr::Bool(v) => {
                Expr::Bool(v)
            }
            parse_ast::Expr::U32(v) => {
                Expr::U32(v)
            },
            parse_ast::Expr::Identifier(id) => {
                println!("Searching for {}", id);
                return Ok(self.name_to_key[&id].clone())
            },
        };

        let id = self.fresh_uid(None);
        let key = self.vars.insert_with(|key| {
            Avis::new(ArgOrVar::Var(key), &id, None)
        });

        self.ssa.insert(key, e);
        Ok(ArgOrVar::Var(key))
    }
}
