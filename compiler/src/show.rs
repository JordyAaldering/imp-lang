use std::mem;

use crate::{arena::{Arena, SecondaryArena}, ast::*};

pub fn show<Ast: AstConfig>(program: &Program<Ast>)-> String {
    program.fundefs.iter()
        .map(|fundef| {
            Show::new().show_fundef(fundef)
        })
        .collect::<Vec<String>>()
        .join("\n\n")
}

pub struct Show<Ast: AstConfig> {
    fargs: Vec<Avis<Ast>>,
    scopes: Vec<(Arena<Avis<Ast>>, SecondaryArena<Expr<Ast>>)>,
}

impl<Ast: AstConfig> Scoped<Ast> for Show<Ast> {
    fn fargs(&self) -> &Vec<Avis<Ast>> {
        &self.fargs
    }

    fn set_fargs(&mut self, fargs: Vec<Avis<Ast>>) {
        self.fargs = fargs
    }

    fn pop_fargs(&mut self) -> Vec<Avis<Ast>> {
        let mut fargs = Vec::new();
        mem::swap(&mut self.fargs, &mut fargs);
        fargs
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
    pub fn new() -> Self {
        Self {
            fargs: Vec::new(),
            scopes: Vec::new(),
        }
    }

    fn indent(&self) -> String {
        " ".repeat(4 * self.scopes.len())
    }

    fn show_fundef(&mut self, fundef: &Fundef<Ast>) -> String {
        self.set_fargs(fundef.args.clone());
        self.push_scope(fundef.ids.clone(), fundef.ssa.clone());
        let mut res = String::new();

        let args = fundef.args.iter()
            .map(|arg| format!("{} {}", arg.ty, arg.name))
            .collect::<Vec<String>>()
            .join(", ");
        let ret_ty = &self.find_id(fundef.ret).unwrap().ty;
        res.push_str(&format!("fn {} ({}) -> {} {{\n", fundef.name, args, ret_ty));

        for (k, id) in fundef.ids.iter() {
            res.push_str(&format!("{}{} {}; // {:?}\n", self.indent(), id.ty, id.name, k));
        }

        for (k, expr) in fundef.ssa.iter() {
            let rhs = self.show_expr(expr);
            let lhs = &self.find_key(k).unwrap().name;
            res.push_str(&format!("{}{} = {};\n", self.indent(), lhs, rhs));
        }

        let ret = &self.find_id(fundef.ret).unwrap().name;
        res.push_str(&format!("    return {};\n", ret));
        res.push_str("}}");

        self.pop_scope();
        self.pop_fargs();
        res
    }

    fn show_expr(&mut self, expr: &Expr<Ast>) -> String {
        use Expr::*;
        match expr {
            Tensor(n) => self.show_tensor(n),
            Binary(n) => self.show_binary(n),
            Unary(n) => self.show_unary(n),
            Bool(v) => v.to_string(),
            U32(v) => v.to_string(),
        }
    }

    fn show_tensor(&mut self, tensor: &Tensor<Ast>) -> String {
        self.push_scope(tensor.ids.clone(), tensor.ssa.clone());
        let mut res = String::new();

        res.push_str("{\n");

        for (k, id) in tensor.ids.iter() {
            res.push_str(&format!("{}{} {}; // {:?}\n", self.indent(), id.ty, id.name, k));
        }

        for (k, expr) in tensor.ssa.iter() {
            let rhs = self.show_expr(expr);
            let lhs = &self.find_key(k).unwrap().name;
            res.push_str(&format!("{}{} = {};\n", self.indent(), lhs, rhs));
        }

        res.push_str(&format!("{}return {};\n",
            self.indent(),
            self.find_id(tensor.ret).unwrap().name,
        ));

        self.pop_scope();

        res.push_str(&format!("{}  | {} <= {} < {} }}",
            self.indent(),
            self.find_id(tensor.lb).unwrap().name,
            self.find_key(tensor.iv.0).unwrap().name,
            self.find_id(tensor.ub).unwrap().name,
        ));

        res
    }

    fn show_binary(&mut self, binary: &Binary) -> String {
        let l = &self.find_id(binary.l).unwrap().name;
        let r = &self.find_id(binary.r).unwrap().name;
        format!("{} {} {}", l, binary.op, r)
    }

    fn show_unary(&mut self, unary: &Unary) -> String {
        let r = &self.find_id(unary.r).unwrap().name;
        format!("{} {}", unary.op, r)
    }
}
