use crate::{ast::{self, ArgOrVar, TypedAst}, compile::compile_ast::*};

pub struct UndoSsa;

impl UndoSsa {
    pub fn new() -> Self {
        Self
    }

    pub fn trav_program(&mut self, program: &ast::Program<TypedAst>) -> Program {
        let fundefs = program.fundefs.iter()
            .map(|f| self.trav_fundef(f))
            .collect();
        Program { fundefs }
    }

    fn trav_fundef(&mut self, fundef: &ast::Fundef<TypedAst>) -> Fundef {
        let args = fundef.args.iter().map(|a| {
            (a.ty.clone(), a.name.clone())
        }).collect();

        let body = self.generate_assignment(&fundef.ret, fundef);

        Fundef {
            name: fundef.name.clone(),
            ret_type: fundef[fundef.ret.clone()].ty.clone(),
            args,
            body: vec![body],
        }
    }

    fn generate_assignment(&mut self, id: &ArgOrVar<TypedAst>, fundef: &ast::Fundef<TypedAst>) -> Stmt {
        let lhs = fundef[id].name.clone();

        let expr = match id {
            ArgOrVar::Arg(i) => {
                Expr::Identifier(fundef.args[*i].name.clone())
            },
            ArgOrVar::Var(k) => {
                // TODO: if an ssa key is used in multiple places, pull the computation out. otherwise inline it
                match &fundef.ssa[*k] {
                    ast::Expr::Tensor(ast::Tensor { iv, expr, lb, ub }) => {
                        let iv = IndexVector(fundef.vars[iv.0].name.clone());
                        let expr = self.inline_expr(expr, fundef);
                        let lb = self.inline_expr(lb, fundef);
                        let ub = self.inline_expr(ub, fundef);
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
                    ast::Expr::Bool(v) => Expr::Bool(*v),
                    ast::Expr::U32(v) => Expr::U32(*v),
                }
            },
            ArgOrVar::IV(k) => {
                Expr::Identifier(fundef.vars[*k].name.clone())
            },
        };

        Stmt::Assign { lhs, expr }
    }

    fn inline_expr(&mut self, id: &ArgOrVar<TypedAst>, fundef: &ast::Fundef<TypedAst>) -> Expr {
        match id {
            ArgOrVar::Arg(i) => {
                Expr::Identifier(fundef.args[*i].name.clone())
            },
            ArgOrVar::Var(k) => {
                println!("looking for {}", fundef.vars[*k].name);
                match &fundef.ssa[*k] {
                    ast::Expr::Tensor(ast::Tensor { iv, expr, lb, ub }) => {
                        let iv = IndexVector(fundef.vars[iv.0].name.clone());
                        let expr = self.inline_expr(expr, fundef);
                        let lb = self.inline_expr(lb, fundef);
                        let ub = self.inline_expr(ub, fundef);
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
                    ast::Expr::Bool(v) => Expr::Bool(*v),
                    ast::Expr::U32(v) => Expr::U32(*v),
                }
            },
            ArgOrVar::IV(k) => {
                Expr::Identifier(fundef.vars[*k].name.clone())
            },
        }
    }
}
