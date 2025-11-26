use std::collections::HashMap;

use slotmap::{SecondaryMap, SlotMap};

use crate::{ast::{self, UntypedAst, VarInfo, VarKey}, scanparse::parse_ast};

pub struct ConvertToSsa {
    uid: usize,
    vars: Option<SlotMap<VarKey, VarInfo<UntypedAst>>>,
    ssa: Option<SecondaryMap<VarKey, ast::Expr>>,
    parse_name_to_key: HashMap<String, VarKey>,
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
            vars: None,
            ssa: None,
            parse_name_to_key: HashMap::new(),
        }
    }

    fn fresh_uid(&mut self, id: Option<&str>) -> String {
        // TODO: generate a name that the user could never write, e.g. something containing `@`
        // This requires us to sanitize these strings again before compiling
        let s = format!("_{}_{}", id.unwrap_or("ssa"), self.uid);
        self.uid += 1;
        s
    }

    pub fn convert_program(&mut self, program: parse_ast::Program) -> SsaResult<ast::Program<UntypedAst>> {
        let mut fundefs = Vec::new();
        for f in program.fundefs {
            fundefs.push(self.convert_fundef(f)?);
        }

        Ok(ast::Program { fundefs })
    }

    pub fn convert_fundef(&mut self, fundef: parse_ast::Fundef) -> SsaResult<ast::Fundef<UntypedAst>> {
        // Reset self
        self.uid = 0;
        self.vars = Some(SlotMap::with_key());
        self.ssa = Some(SecondaryMap::new());
        self.parse_name_to_key.clear();

        let mut args = Vec::new();
        for (ty, id) in fundef.args {
            let key = self.vars.as_mut().unwrap().insert_with_key(|key| {
                VarInfo::new(key, &id, Some(ty))
            });
            args.push(key);
            self.parse_name_to_key.insert(id, key);
        }

        let ret_value = self.convert_body(fundef.body)?;
        self.vars.as_mut().unwrap()[ret_value].set_type(fundef.ret_type);

        Ok(ast::Fundef {
            id: fundef.id,
            args,
            vars: self.vars.take().unwrap(),
            ssa: self.ssa.take().unwrap(),
            ret_value,
        })
    }

    pub fn convert_body(&mut self, body: Vec<parse_ast::Stmt>) -> SsaResult<VarKey> {
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

    pub fn convert_stmt(&mut self, stmt: parse_ast::Stmt) -> SsaResult<Option<VarKey>> {
        match stmt {
            parse_ast::Stmt::Assign { lhs, expr } => {
                // We need explicit handling of the outermost expression, which is why we don't call convert_expr immediately
                // We can probably make this nicer though
                let e = match expr {
                    parse_ast::Expr::Binary { l, r, op } => {
                        let l_key = self.convert_expr(*l)?;
                        let r_key = self.convert_expr(*r)?;
                        ast::Expr::Binary(ast::Binary { l: l_key, r: r_key, op })
                    },
                    parse_ast::Expr::Unary { r, op } => {
                        let r_key = self.convert_expr(*r)?;
                        ast::Expr::Unary(ast::Unary { r: r_key, op })
                    },
                    parse_ast::Expr::Identifier(id) => {
                        println!("Found an rhs identifier: {}", id);
                        return Ok(Some(self.parse_name_to_key[&id]));
                    },
                    parse_ast::Expr::Bool(v) => {
                        ast::Expr::Bool(v)
                    }
                    parse_ast::Expr::U32(v) => {
                        ast::Expr::U32(v)
                    },
                };

                let id = self.fresh_uid(Some(&lhs));
                let key = self.vars.as_mut().unwrap().insert_with_key(|key| {
                    VarInfo::new(key, &id, None)
                });

                self.ssa.as_mut().unwrap().insert(key, e);

                self.parse_name_to_key.insert(lhs, key);

                Ok(None)
            },
            parse_ast::Stmt::Return { expr } => {
                let ret_value_key = self.convert_expr(expr)?;
                Ok(Some(ret_value_key))
            },
        }
    }

    pub fn convert_expr(&mut self, expr: parse_ast::Expr) -> SsaResult<VarKey> {
        if let parse_ast::Expr::Identifier(id) = &expr {
            return Ok(self.parse_name_to_key[id])
        }

        let e = match expr {
            parse_ast::Expr::Binary { l, r, op } => {
                let l_key = self.convert_expr(*l)?;
                let r_key = self.convert_expr(*r)?;
                ast::Expr::Binary(ast::Binary { l: l_key, r: r_key, op })
            },
            parse_ast::Expr::Unary { r, op } => {
                let r_key = self.convert_expr(*r)?;
                ast::Expr::Unary(ast::Unary { r: r_key, op })
            },
            parse_ast::Expr::Identifier(_) => {
                unreachable!()
            },
            parse_ast::Expr::Bool(v) => {
                ast::Expr::Bool(v)
            }
            parse_ast::Expr::U32(v) => {
                ast::Expr::U32(v)
            },
        };

        let id = self.fresh_uid(None);
        let key = self.vars.as_mut().unwrap().insert_with_key(|key| {
            VarInfo::new(key, &id, None)
        });

        let prev_key = self.ssa.as_mut().unwrap().insert(key, e);
        // Check that the inserted key was indeed unique
        debug_assert!(prev_key.is_none());
        Ok(key)
    }
}
