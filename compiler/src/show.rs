use crate::{ast::*, traverse::AstPass};

/// Pretty-print an AST back to source code.
///
/// Uses AstPass to recursively render the AST. Works on any AST variant
/// (UntypedAst, TypedAst, etc.) and outputs formatted code.
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
                    self.pass_expr((**expr).clone());
                    self.output
                        .push_str(&format!("    {} = <expr>;\n", avis.name));
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

    fn pass_expr(&mut self, expr: Expr<'ast, Ast>) -> Expr<'ast, Ast> {
        expr
    }

    fn pass_ssa(&mut self, id: ArgOrVar<'ast, Ast>) -> ArgOrVar<'ast, Ast> {
        id
    }

    fn pass_tensor(&mut self, tensor: Tensor<'ast, Ast>) -> Tensor<'ast, Ast> {
        let mut out = String::new();
        self.level += 1;
        let indent = self.indent();

        out.push_str("{\n");

        for stmt in &tensor.ssa {
            if let Stmt::Assign { avis, expr } = stmt {
                self.pass_expr((**expr).clone());
                out.push_str(&format!("{}{} = <expr>;\n", indent, avis.name));
            }
        }

        out.push_str(&format!("{}return <val>;\n", indent));

        out.push_str(&format!(
            "{}| {} <= {} < {} }}",
            indent, "<lb>", tensor.iv.name, "<ub>"
        ));

        self.level -= 1;
        tensor
    }

    fn pass_binary(&mut self, binary: Binary<'ast, Ast>) -> Binary<'ast, Ast> {
        binary
    }

    fn pass_unary(&mut self, unary: Unary<'ast, Ast>) -> Unary<'ast, Ast> {
        unary
    }

    fn pass_bool(&mut self, value: bool) -> bool {
        value
    }

    fn pass_u32(&mut self, value: u32) -> u32 {
        value
    }
}
