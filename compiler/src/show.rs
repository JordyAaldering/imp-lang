use crate::{ast::*, traverse::AstPass};

pub fn show<'ast, Ast: AstConfig>(program: &Program<'ast, Ast>) -> String {
    let mut shower = Show::<'ast, Ast>::new();
    shower.pass_program(program.clone());
    shower.finish()
}

struct Show<'ast, Ast: AstConfig> {
    output: String,
    level: usize,
    args: Vec<&'ast Avis<Ast>>,
    _marker: std::marker::PhantomData<&'ast Ast>,
}

impl<'ast, Ast: AstConfig> Show<'ast, Ast> {
    fn new() -> Self {
        Self {
            output: String::new(),
            level: 0,
            args: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    fn finish(self) -> String {
        self.output
    }

    fn indent(&self) -> String {
        " ".repeat(4 * self.level)
    }

    fn name_of(&self, id: ArgOrVar<'ast, Ast>) -> String {
        match id {
            ArgOrVar::Arg(i) => self.args[i].name.clone(),
            ArgOrVar::Var(v) => v.name.clone(),
        }
    }
}

impl<'ast, Ast: AstConfig> AstPass<'ast> for Show<'ast, Ast> {
    type InAst = Ast;
    type OutAst = Ast;
    type ExprOk = String;

    fn pass_program(&mut self, program: Program<'ast, Ast>) -> Program<'ast, Ast> {
        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            let fundef = self.pass_fundef(fundef);
            if !self.output.is_empty() {
                self.output.push_str("\n\n");
            }
            fundefs.push(fundef);
        }

        Program { fundefs }
    }

    fn pass_fundef(&mut self, fundef: Fundef<'ast, Ast>) -> Fundef<'ast, Ast> {
        self.args = fundef.args.clone();

        let args_str = fundef
            .args
            .iter()
            .map(|arg| format!("{} {}", arg.ty, arg.name))
            .collect::<Vec<_>>()
            .join(", ");

        let ret_id = fundef.ret_id();
        self.output
            .push_str(&format!(
                "fn {}({}) -> {} {{\n",
                fundef.name,
                args_str,
                fundef.typof(ret_id)
            ));

        for id in &fundef.ids {
            self.output
                .push_str(&format!("    {} {};\n", id.ty, id.name));
        }

        for stmt in &fundef.body {
            match stmt {
                Stmt::Assign { avis, expr } => {
                    let (expr_str, _) = self.pass_expr((**expr).clone());
                    self.output
                        .push_str(&format!("    {} = {};\n", avis.name, expr_str));
                }
                Stmt::Return { id } => {
                    self.output
                        .push_str(&format!("    return {};\n", self.name_of(*id)));
                }
                Stmt::Index { .. } => {}
            }
        }

        self.output.push_str("}");
        fundef
    }

    fn pass_expr(&mut self, expr: Expr<'ast, Ast>) -> (String, Expr<'ast, Ast>) {
        use Expr::*;
        match expr {
            Tensor(n) => {
                let (s, n) = self.pass_tensor(n);
                (s, Tensor(n))
            }
            Binary(n) => {
                let (s, n) = self.pass_binary(n);
                (s, Binary(n))
            }
            Unary(n) => {
                let (s, n) = self.pass_unary(n);
                (s, Unary(n))
            }
            Bool(n) => {
                let (s, n) = self.pass_bool(n);
                (s, Bool(n))
            }
            U32(n) => {
                let (s, n) = self.pass_u32(n);
                (s, U32(n))
            }
        }
    }

    fn pass_ssa(&mut self, id: ArgOrVar<'ast, Ast>) -> (String, ArgOrVar<'ast, Ast>) {
        (self.name_of(id), id)
    }

    fn pass_tensor(&mut self, tensor: Tensor<'ast, Ast>) -> (String, Tensor<'ast, Ast>) {
        let mut out = String::new();
        self.level += 1;
        let indent = self.indent();

        out.push_str("{\n");

        for stmt in &tensor.ssa {
            if let Stmt::Assign { avis, expr } = stmt {
                let (expr_str, _) = self.pass_expr((**expr).clone());
                out.push_str(&format!("{}{} = {};\n", indent, avis.name, expr_str));
            }
        }

        let (ret_str, _) = self.pass_ssa(tensor.ret);
        out.push_str(&format!("{}return {};\n", indent, ret_str));

        let (lb_str, _) = self.pass_ssa(tensor.lb);
        let (ub_str, _) = self.pass_ssa(tensor.ub);
        out.push_str(&format!(
            "{}| {} <= {} < {} }}",
            indent, lb_str, tensor.iv.name, ub_str
        ));

        self.level -= 1;
        (out, tensor)
    }

    fn pass_binary(&mut self, binary: Binary<'ast, Ast>) -> (String, Binary<'ast, Ast>) {
        let (l_str, _) = self.pass_ssa(binary.l);
        let (r_str, _) = self.pass_ssa(binary.r);
        (format!("{} {} {}", l_str, binary.op, r_str), binary)
    }

    fn pass_unary(&mut self, unary: Unary<'ast, Ast>) -> (String, Unary<'ast, Ast>) {
        let (r_str, _) = self.pass_ssa(unary.r);
        (format!("{} {}", unary.op, r_str), unary)
    }

    fn pass_bool(&mut self, value: bool) -> (String, bool) {
        (value.to_string(), value)
    }

    fn pass_u32(&mut self, value: u32) -> (String, u32) {
        (value.to_string(), value)
    }
}
