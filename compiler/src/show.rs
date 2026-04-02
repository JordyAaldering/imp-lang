use crate::{ast::*, Visit};

pub fn show<'ast, Ast: AstConfig>(program: &Program<'ast, Ast>) -> String {
    let mut show = Show::new();
    show.visit_program(program);
    show.output
}

struct Show<'ast, Ast: AstConfig> {
    args: Vec<&'ast Avis<Ast>>,
    depth: usize,
    output: String,
}

impl<'ast, Ast: AstConfig> Show<'ast, Ast> {
    fn new() -> Self {
        Self {
            args: Vec::new(),
            output: String::new(),
            depth: 0,
        }
    }

    fn push(&mut self, s: &str) {
        self.indent();
        self.output.push_str(s);
    }

    fn indent(&mut self) {
        self.output.push_str(&" ".repeat(4 * self.depth));
    }
}

impl<'ast, Ast: AstConfig + 'ast> Visit<'ast> for Show<'ast, Ast> {
    type Ast = Ast;

    fn visit_program(&mut self, program: &Program<'ast, Ast>) {
        for fundef in &program.fundefs {
            self.visit_fundef(fundef);
            self.output.push('\n');
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, Ast>) {
        self.args = fundef.args.clone();

        self.push(&format!("fn {}(", fundef.name));

        self.visit_fargs(&fundef.args);

        self.output.push_str(&format!(") -> {} {{\n", fundef.ret_type));

        self.depth += 1;

        for id in &fundef.decs {
            self.push(&format!("{} {};\n", id.ty, id.name));
        }

        for stmt in &fundef.body {
            self.visit_stmt(stmt);
        }

        self.depth -= 1;

        self.push("}");
    }

    fn visit_farg(&mut self, arg: &'ast Avis<Self::Ast>) {
        self.output.push_str(&format!("{} {}, ", arg.ty, arg.name));
    }

    fn visit_stmt(&mut self, stmt: &Stmt<'ast, Self::Ast>) {
        use Stmt::*;
        match stmt {
            Assign(n) => self.visit_assign(n),
            Return(n) => self.visit_return(n),
        }
        self.output.push_str(";\n");
    }

    fn visit_assign(&mut self, assign: &Assign<'ast, Self::Ast>) {
        self.push(&assign.avis.name);
        self.output.push_str(" = ");
        self.visit_expr(&assign.expr);
    }

    fn visit_return(&mut self, ret: &Return<'ast, Self::Ast>) {
        self.push("return ");
        self.visit_id(&ret.id);
    }

    fn visit_expr(&mut self, expr: &Expr<'ast, Self::Ast>) {
        use Expr::*;
        match expr {
            Tensor(n) => self.visit_tensor(n),
            Binary(n) => self.visit_binary(n),
            Unary(n) => self.visit_unary(n),
            Id(n) => self.visit_id(n),
            Bool(n) => self.visit_bool(n),
            U32(n) => self.visit_u32(n),
        }
    }

    fn visit_tensor(&mut self, tensor: &Tensor<'ast, Self::Ast>) {
        self.output.push_str("{\n");

        self.depth += 1;

        for stmt in &tensor.body {
            self.visit_stmt(stmt);
        }

        self.indent();
        self.visit_id(&tensor.ret);
        self.output.push('\n');

        self.depth -= 1;

        self.push(&"| ");
        self.visit_id(&tensor.lb);
        self.output.push_str(" <= ");
        self.output.push_str(&tensor.iv.name);
        self.output.push_str(" < ");
        self.visit_id(&tensor.ub);
        self.output.push_str(&" }");

    }

    fn visit_binary(&mut self, binary: &Binary<'ast, Self::Ast>) {
        self.visit_id(&binary.l);
        self.output.push(' ');
        self.output.push_str(&binary.op.to_string());
        self.output.push(' ');
        self.visit_id(&binary.r);
    }

    fn visit_unary(&mut self, unary: &Unary<'ast, Self::Ast>) {
        self.output.push_str(&unary.op.to_string());
        self.visit_id(&unary.r);
    }

    fn visit_id(&mut self, id: &Id<'ast, Self::Ast>) {
        match id {
            Id::Arg(i) => self.output.push_str(&self.args[*i].name),
            Id::Var(v) => self.output.push_str(&v.name),
        }
    }

    fn visit_bool(&mut self, value: &bool) {
        self.output.push_str(&value.to_string());
    }

    fn visit_u32(&mut self, value: &u32) {
        self.output.push_str(&value.to_string());
    }
}
