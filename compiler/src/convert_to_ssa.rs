use std::collections::HashMap;

use crate::{arena::{Arena, SecondaryArena}, ast::*, scanparse::parse_ast};

pub struct ConvertToSsa {
    uid: usize,
    ids: Vec<Arena<Avis<UntypedAst>>>,
    ssa: Vec<SecondaryArena<Expr<UntypedAst>>>,
    name_to_key: Vec<HashMap<String, ArgOrVar>>,
}

pub fn convert_to_ssa(program: parse_ast::Program) -> Program<UntypedAst> {
    let fundefs = program.fundefs.into_iter()
        .map(|f| ConvertToSsa::new().convert_fundef(f))
        .collect();
    Program { fundefs }
}

impl ConvertToSsa {
    fn new() -> Self {
        Self {
            uid: 0,
            ids: Vec::new(),
            ssa: Vec::new(),
            name_to_key: Vec::new(),
        }
    }

    fn fresh_uid(&mut self) -> String {
        self.uid += 1;
        format!("_ssa_{}", self.uid)
    }

    fn create_scope(&mut self) {
        self.ids.push(Arena::new());
        self.ssa.push(SecondaryArena::new());
        self.name_to_key.push(HashMap::new());
    }

    fn close_scope(&mut self) -> (Arena<Avis<UntypedAst>>, SecondaryArena<Expr<UntypedAst>>) {
        self.name_to_key.pop().unwrap();
        let ids = self.ids.pop().unwrap();
        let ssa = self.ssa.pop().unwrap();
        (ids, ssa)
    }

    pub fn convert_fundef(&mut self, fundef: parse_ast::Fundef) -> Fundef<UntypedAst> {
        self.create_scope();

        let mut args = Vec::new();
        for (i, (ty, id)) in fundef.args.into_iter().enumerate() {
            args.push(Avis::new(ArgOrVar::Arg(i), &id, Some(ty)));
            self.name_to_key.last_mut().unwrap().insert(id, ArgOrVar::Arg(i));
        }

        for stmt in fundef.body {
            self.convert_stmt(stmt);
        }

        let ret_value = self.convert_expr(fundef.ret_expr);
        if let ArgOrVar::Var(k) = ret_value {
            self.ids[0][k].ty = Some(fundef.ret_type);
        }

        let (ids, ssa) = self.close_scope();

        Fundef {
            name: fundef.id,
            args,
            ids,
            ssa,
            ret: ret_value,
        }
    }

    pub fn convert_stmt(&mut self, stmt: parse_ast::Stmt) {
        match stmt {
            parse_ast::Stmt::Assign { lhs, expr } => {
                let key = self.convert_expr(expr);
                self.name_to_key.last_mut().unwrap().insert(lhs, key);
            },
        }
    }

    pub fn convert_expr(&mut self, expr: parse_ast::Expr) -> ArgOrVar {
        let e = match expr {
            parse_ast::Expr::Tensor { expr, iv, lb, ub } => {
                let lb = self.convert_expr(*lb);
                let ub = self.convert_expr(*ub);

                self.ids.push(Arena::new());
                self.ssa.push(SecondaryArena::new());
                self.name_to_key.push(HashMap::new());

                let key = self.ids.last_mut().unwrap().insert_with(|key| {
                    Avis::new(ArgOrVar::Iv(key), &iv.0, None)
                });
                self.name_to_key.last_mut().unwrap().insert(iv.0.clone(), ArgOrVar::Iv(key));
                let iv = IndexVector(key);

                let ret = self.convert_expr(*expr);

                let ids = self.ids.pop().unwrap();
                let ssa = self.ssa.pop().unwrap();
                self.name_to_key.pop().unwrap();

                let body = Block { ids, ssa, ret };
                Expr::Tensor(Tensor { body, iv, lb, ub })
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
                for scope in self.name_to_key.iter().rev() {
                    if let Some(key) = scope.get(&id) {
                        return *key;
                    }
                }
                unreachable!("could not find {}", id);
            },
        };

        let id = self.fresh_uid();
        let key = self.ids.last_mut().unwrap().insert_with(|key| {
            Avis::new(ArgOrVar::Var(key), &id, None)
        });

        self.ssa.last_mut().unwrap().insert(key, e);
        ArgOrVar::Var(key)
    }
}
