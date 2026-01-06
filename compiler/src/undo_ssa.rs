use slotmap::{SecondaryMap, SlotMap};

use crate::{ast::{self, ArgOrVar, Avis, TypedAst, TypedKey}, compile::compile_ast::*};

pub struct UndoSsa {
    args: Vec<Avis<TypedAst>>,
    ids: SlotMap<TypedKey, ast::Avis<TypedAst>>,
    scopes: Vec<SecondaryMap<TypedKey, ast::Expr<TypedAst>>>,
}

impl UndoSsa {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            ids: SlotMap::with_key(),
            scopes: Vec::new(),
        }
    }

    fn find(&self, key: ArgOrVar<TypedAst>) -> &Avis<TypedAst> {
        match key {
            ArgOrVar::Arg(i) => &self.args[i],
            ArgOrVar::Var(k) => &self.ids[k],
            ArgOrVar::Iv(k) => &self.ids[k],
        }
    }

    fn find_ssa(&self, key: TypedKey) -> &ast::Expr<TypedAst> {
        for scope in self.scopes.iter().rev() {
            if let Some(expr) = scope.get(key) {
                return expr;
            }
        }
        unreachable!()
    }

    pub fn trav_program(&mut self, program: &ast::Program<TypedAst>) -> Program {
        let fundefs = program.fundefs.iter()
            .map(|f| self.trav_fundef(f))
            .collect();
        Program { fundefs }
    }

    fn trav_fundef(&mut self, fundef: &ast::Fundef<TypedAst>) -> Fundef {
        self.args = fundef.args.clone();
        self.ids = fundef.ids.clone();
        self.scopes.push(fundef.ssa.clone());

        let args = fundef.args.iter().map(|a| {
            (a.ty.clone(), a.name.clone())
        }).collect();

        let mut body = Vec::new();
        body.push(self.generate_assignment(fundef.ret, fundef));
        body.push(Stmt::Return { expr: Expr::Identifier(self.find(fundef.ret).name.to_owned()) });

        self.scopes.pop().unwrap();
        assert!(self.scopes.is_empty());
        Fundef {
            name: fundef.name.to_owned(),
            ret_type: self.find(fundef.ret).ty.to_owned(),
            args,
            block: Block { stmts: body },
        }
    }

    fn generate_assignment(&mut self, id: ArgOrVar<TypedAst>, fundef: &ast::Fundef<TypedAst>) -> Stmt {
        let lhs = self.find(id).name.clone();

        let expr = match id {
            ArgOrVar::Arg(i) => {
                Expr::Identifier(fundef.args[i].name.clone())
            },
            ArgOrVar::Var(k) => {
                match self.find_ssa(k).clone() {
                    ast::Expr::Tensor(ast::Tensor { iv, lb, ub, ret, ssa }) => {
                        self.scopes.push(ssa.clone());
                        let iv = IndexVector(fundef.ids[iv].name.clone());
                        let expr = self.inline_expr(ret, fundef);
                        let lb = self.inline_expr(lb, fundef);
                        let ub = self.inline_expr(ub, fundef);
                        self.scopes.pop().unwrap();
                        Expr::Tensor { iv, expr: Box::new(expr), lb: Box::new(lb), ub: Box::new(ub) }
                    },
                    ast::Expr::Binary(ast::Binary { l, r, op }) => {
                        let l = self.inline_expr(l, fundef);
                        let r = self.inline_expr(r, fundef);
                        Expr::Binary { l: Box::new(l), r: Box::new(r), op: op.clone() }
                    },
                    ast::Expr::Unary(ast::Unary { r, op }) => {
                        let r = self.inline_expr(r, fundef);
                        Expr::Unary { r: Box::new(r), op: op.clone() }
                    },
                    ast::Expr::Bool(v) => Expr::Bool(v),
                    ast::Expr::U32(v) => Expr::U32(v),
                }
            },
            ArgOrVar::Iv(k) => {
                Expr::Identifier(fundef.ids[k].name.clone())
            },
        };

        Stmt::Assign { lhs, expr }
    }

    fn inline_expr(&mut self, id: ArgOrVar<TypedAst>, fundef: &ast::Fundef<TypedAst>) -> Expr {
        match id {
            ArgOrVar::Arg(i) => {
                Expr::Identifier(fundef.args[i].name.clone())
            },
            ArgOrVar::Var(k) => {
                match self.find_ssa(k).clone() {
                    ast::Expr::Tensor(ast::Tensor { iv, lb, ub, ret, ssa }) => {
                        self.scopes.push(ssa.clone());
                        let iv = IndexVector(fundef.ids[iv].name.clone());
                        let expr = self.inline_expr(ret, fundef);
                        let lb = self.inline_expr(lb, fundef);
                        let ub = self.inline_expr(ub, fundef);
                        self.scopes.pop().unwrap();
                        Expr::Tensor { iv, expr: Box::new(expr), lb: Box::new(lb), ub: Box::new(ub) }
                    },
                    ast::Expr::Binary(ast::Binary { l, r, op }) => {
                        let l = self.inline_expr(l, fundef);
                        let r = self.inline_expr(r, fundef);
                        Expr::Binary { l: Box::new(l), r: Box::new(r), op: op.clone() }
                    },
                    ast::Expr::Unary(ast::Unary { r, op }) => {
                        let r = self.inline_expr(r, fundef);
                        Expr::Unary { r: Box::new(r), op: op.clone() }
                    },
                    ast::Expr::Bool(v) => Expr::Bool(v),
                    ast::Expr::U32(v) => Expr::U32(v),
                }
            },
            ArgOrVar::Iv(k) => {
                Expr::Identifier(fundef.ids[k].name.clone())
            },
        }
    }
}
