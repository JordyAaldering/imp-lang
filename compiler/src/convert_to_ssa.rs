use std::collections::HashMap;

use crate::{arena::{Arena, SecondaryArena}, ast::*, scanparse::parse_ast};

pub fn convert_to_ssa(program: parse_ast::Program) -> Program<UntypedAst> {
    let fundefs = program.fundefs.into_iter()
        .map(|f| ConvertToSsa::new().convert_fundef(f))
        .collect();
    Program { fundefs }
}

pub struct ConvertToSsa {
    uid: usize,
    scopes: Vec<(Arena<Avis<UntypedAst>>, SecondaryArena<Expr<UntypedAst>>)>,
    name_to_key: Vec<HashMap<String, ArgOrVar>>,
}

impl Scoped<UntypedAst> for ConvertToSsa {
    fn fargs(&self) -> &Vec<Avis<UntypedAst>> {
        unreachable!()
    }

    fn set_fargs(&mut self, _fargs: Vec<Avis<UntypedAst>>) {
        unreachable!()
    }

    fn pop_fargs(&mut self) -> Vec<Avis<UntypedAst>> {
        unreachable!()
    }

    fn scopes(&self) -> &Vec<(Arena<Avis<UntypedAst>>, SecondaryArena<Expr<UntypedAst>>)> {
        &self.scopes
    }

    fn push_scope(&mut self, ids: Arena<Avis<UntypedAst>>, ssa: SecondaryArena<Expr<UntypedAst>>) {
        self.name_to_key.push(HashMap::new());
        self.scopes.push((ids, ssa));
    }

    fn pop_scope(&mut self) -> (Arena<Avis<UntypedAst>>, SecondaryArena<Expr<UntypedAst>>) {
        self.name_to_key.pop().unwrap();
        self.scopes.pop().unwrap()
    }
}

impl ConvertToSsa {
    fn new() -> Self {
        Self {
            uid: 0,
            scopes: Vec::new(),
            name_to_key: Vec::new(),
        }
    }

    fn fresh_uid(&mut self) -> String {
        self.uid += 1;
        format!("_ssa_{}", self.uid)
    }

    pub fn convert_fundef(&mut self, fundef: parse_ast::Fundef) -> Fundef<UntypedAst> {
        self.push_scope(Arena::new(), SecondaryArena::new());

        let mut args = Vec::new();
        for (i, (ty, id)) in fundef.args.into_iter().enumerate() {
            args.push(Avis::new(ArgOrVar::Arg(i), &id, MaybeType(Some(ty))));
            self.name_to_key.last_mut().unwrap().insert(id, ArgOrVar::Arg(i));
        }

        for stmt in fundef.body {
            self.convert_stmt(stmt);
        }

        let ret_value = self.convert_expr(fundef.ret_expr);
        if let ArgOrVar::Var(k) = ret_value {
            self.scopes[0].0[k].ty = MaybeType(Some(fundef.ret_type));
        }

        let (ids, ssa) = self.pop_scope();

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

                self.push_scope(Arena::new(), SecondaryArena::new());

                let key = self.scopes.last_mut().unwrap().0.insert_with(|key| {
                    Avis::new(ArgOrVar::Iv(key), &iv, MaybeType(None))
                });
                self.name_to_key.last_mut().unwrap().insert(iv.clone(), ArgOrVar::Iv(key));
                let iv = IndexVector(key);

                let ret = self.convert_expr(*expr);

                let (ids, ssa) = self.pop_scope();

                Expr::Tensor(Tensor { iv, lb, ub, ids, ssa, ret })
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
        let key = self.scopes.last_mut().unwrap().0.insert_with(|key| {
            Avis::new(ArgOrVar::Var(key), &id, MaybeType(None))
        });

        self.scopes.last_mut().unwrap().1.insert(key, e);
        ArgOrVar::Var(key)
    }
}
