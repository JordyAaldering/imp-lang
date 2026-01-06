use slotmap::{DefaultKey, SecondaryMap, SlotMap};

use crate::ast::*;

pub fn show<Ast: AstConfig>(program: &Program<Ast>)-> String {
    program.fundefs.iter()
        .map(|fundef| {
            Show::new().show_fundef(fundef)
        })
        .collect::<Vec<String>>()
        .join("\n\n")
}

pub struct Show<Ast: AstConfig> {
    args: Vec<Avis<Ast>>,
    ids: SlotMap<DefaultKey, Avis<Ast>>,
    scopes: Vec<SecondaryMap<DefaultKey, Expr<Ast>>>,
}

impl<Ast: AstConfig> Show<Ast> {
    /// TODO: probably should create some init variant instead that has the fundef as its argument
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            ids: SlotMap::new(),
            scopes: Vec::new(),
        }
    }

    fn indent(&self) -> String {
        " ".repeat(4 * self.scopes.len())
    }

    fn find(&self, key: ArgOrVar) -> &Avis<Ast> {
        match key {
            ArgOrVar::Arg(i) => &self.args[i],
            ArgOrVar::Var(k) => &self.ids[k],
            ArgOrVar::Iv(k) => &self.ids[k],
        }
    }

    fn show_fundef(&mut self, fundef: &Fundef<Ast>) -> String {
        self.args = fundef.args.clone();
        self.ids = fundef.ids.clone();
        self.scopes.push(fundef.ssa.clone());

        let mut res = String::new();

        let args = fundef.args.iter()
            .map(|arg| format!("{} {}", arg.ty, arg.name))
            .collect::<Vec<String>>()
            .join(", ");
        let ret_ty = &self.find(fundef.ret).ty;
        res.push_str(&format!("fn {}({}) -> {} {{\n", fundef.name, args, ret_ty));

        for (k, id) in fundef.ids.iter() {
            res.push_str(&format!("{}{} {}; // {:?}\n", self.indent(), id.ty, id.name, k));
        }

        for (k, expr) in fundef.ssa.iter() {
            let rhs = self.show_expr(expr);
            let lhs = &self.ids[k].name;
            res.push_str(&format!("{}{} = {};\n", self.indent(), lhs, rhs));
        }

        let ret = &self.find(fundef.ret).name;
        res.push_str(&format!("    return {};\n", ret));
        res.push_str("}}");

        self.scopes.pop().unwrap();
        assert!(self.scopes.is_empty());
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
        self.scopes.push(tensor.ssa.clone());
        let mut res = String::new();

        res.push_str("{\n");

        for (k, expr) in tensor.ssa.iter() {
            let rhs = self.show_expr(expr);
            let lhs = &self.ids[k].name;
            res.push_str(&format!("{}{} = {};\n", self.indent(), lhs, rhs));
        }

        res.push_str(&format!("{}return {};\n",
            self.indent(),
            self.find(tensor.ret).name,
        ));

        res.push_str(&format!("{}| {} <= {} < {} }}",
            self.indent(),
            self.find(tensor.lb).name,
            self.ids[tensor.iv].name,
            self.find(tensor.ub).name,
        ));

        self.scopes.pop().unwrap();

        res
    }

    fn show_binary(&mut self, binary: &Binary) -> String {
        let l = &self.find(binary.l).name;
        let r = &self.find(binary.r).name;
        format!("{} {} {}", l, binary.op, r)
    }

    fn show_unary(&mut self, unary: &Unary) -> String {
        let r = &self.find(unary.r).name;
        format!("{} {}", unary.op, r)
    }
}
