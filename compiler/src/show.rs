use crate::{ast::*, traverse::Visit};

/// Pretty-print an AST back to source code.
///
/// Uses AstVisit to recursively render the AST. Works on any AST variant
/// (UntypedAst, TypedAst, etc.) and outputs formatted code.
pub fn show<'ast, Ast: AstConfig>(program: &Program<'ast, Ast>) -> String {
    let mut shower = Show::<'ast, Ast>::new();
    shower.visit_program(program.clone());
    shower.finish()
}

struct Show<'ast, Ast: AstConfig> {
    output: String,
    args: Vec<&'ast Avis<Ast>>,
    _marker: std::marker::PhantomData<&'ast Ast>,
}

impl<'ast, Ast: AstConfig> Show<'ast, Ast> {
    fn new() -> Self {
        Self {
            output: String::new(),
            args: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    fn finish(self) -> String {
        self.output
    }

    fn name_of(&self, id: Id<'ast, Ast>) -> String {
        match id {
            Id::Arg(i) => self.args[i].name.clone(),
            Id::Var(v) => v.name.clone(),
        }
    }

    fn indent(level: usize) -> String {
        " ".repeat(4 * level)
    }

    fn format_expr(&self, expr: &Expr<'ast, Ast>, level: usize) -> String {
        match expr {
            Expr::Tensor(tensor) => self.format_tensor(tensor, level),
            Expr::Binary(Binary { l, r, op }) => {
                format!("{} {} {}", self.name_of(*l), op, self.name_of(*r))
            }
            Expr::Unary(Unary { r, op }) => {
                format!("{} {}", op, self.name_of(*r))
            }
            Expr::Bool(v) => v.to_string(),
            Expr::U32(v) => v.to_string(),
        }
    }

    fn format_tensor(&self, tensor: &Tensor<'ast, Ast>, level: usize) -> String {
        let inner = level + 1;
        let inner_indent = Self::indent(inner);

        let mut out = String::new();
        out.push_str("{\n");

        for stmt in &tensor.body {
            match stmt {
                Stmt::Assign(Assign { avis, expr }) => {
                    let rhs = self.format_expr(expr, inner);
                    out.push_str(&format!("{}{} = {};\n", inner_indent, avis.name, rhs));
                }
                Stmt::Return(Return { id }) => {
                    out.push_str(&format!("{}return {};\n", inner_indent, self.name_of(*id)));
                }
            }
        }

        out.push_str(&format!(
            "{}| {} <= {} < {} }}",
            inner_indent,
            self.name_of(tensor.lb),
            tensor.iv.name,
            self.name_of(tensor.ub)
        ));

        out
    }
}

impl<'ast, Ast: AstConfig + 'ast> Visit<'ast> for Show<'ast, Ast> {
    type Ast = Ast;

    fn visit_program(&mut self, program: Program<'ast, Ast>) -> Program<'ast, Ast> {
        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            let fundef = self.visit_fundef(fundef);
            if !self.output.is_empty() {
                self.output.push_str("\n\n");
            }
            fundefs.push(fundef);
        }

        Program { fundefs }
    }

    fn visit_fundef(&mut self, fundef: Fundef<'ast, Ast>) -> Fundef<'ast, Ast> {
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

        for id in &fundef.decls {
            self.output
                .push_str(&format!("    {} {};\n", id.ty, id.name));
        }

        for stmt in &fundef.body {
            match stmt {
                Stmt::Assign(Assign { avis, expr }) => {
                    let expr_str = self.format_expr(expr, 1);
                    self.output
                        .push_str(&format!("    {} = {};\n", avis.name, expr_str));
                }
                Stmt::Return(Return { id }) => {
                    self.output
                        .push_str(&format!("    return {};\n", self.name_of(*id)));
                }
            }
        }

        self.output.push_str("}");
        fundef
    }
}
