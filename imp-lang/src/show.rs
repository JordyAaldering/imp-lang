use crate::{ast::*, Visit};

pub fn show<'ast, Ast: AstConfig + 'ast>(program: &Program<'ast, Ast>) -> String {
    let mut show: Show<'ast, Ast> = Show::new();
    show.visit_program(program);
    show.output
}

struct Show<'ast, Ast: AstConfig> {
    args: Vec<&'ast Farg>,
    depth: usize,
    output: String,
    _phantom: std::marker::PhantomData<Ast>,
}

impl<'ast, Ast: AstConfig> Show<'ast, Ast> {
    fn new() -> Self {
        Self {
            args: Vec::new(),
            output: String::new(),
            depth: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn indent(&mut self) {
        self.output.push_str(&" ".repeat(4 * self.depth));
    }
}

impl<'ast, Ast: AstConfig + 'ast> Visit<'ast> for Show<'ast, Ast> {
    type Ast = Ast;

    fn visit_program(&mut self, program: &Program<'ast, Self::Ast>) {
        for fundef in &program.fundefs {
            self.visit_fundef(fundef);
            self.write("\n");
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, Self::Ast>) {
        self.args = fundef.args.clone();

        self.write(&format!("fn {}(", fundef.name));
        self.visit_fargs(&fundef.args);
        self.write(") -> ");
        self.visit_type(&fundef.ret_type);
        self.write(" {\n");

        self.depth += 1;
        for id in &fundef.decs {
            self.indent();
            Self::Ast::visit_type(self, &id.ty);
            self.write(&format!(" {};\n", id.name));
        }
        for stmt in &fundef.body {
            self.visit_stmt(stmt);
        }
        self.depth -= 1;

        self.write("}");
    }

    fn visit_farg(&mut self, arg: &'ast Farg) {
        self.visit_type(&arg.ty);
        self.write(&format!(" {}, ", arg.name));
    }

    fn visit_stmt(&mut self, stmt: &Stmt<'ast, Self::Ast>) {
        match stmt {
            Stmt::Assign(n) => self.visit_assign(n),
            Stmt::Return(n) => self.visit_return(n),
        }
        self.write(";\n");
    }

    fn visit_assign(&mut self, assign: &Assign<'ast, Self::Ast>) {
        self.indent();
        self.write(&assign.lvis.name);
        self.write(" = ");
        self.visit_expr(assign.expr);
    }

    fn visit_return(&mut self, ret: &Return<'ast, Self::Ast>) {
        self.indent();
        self.write("return ");
        self.visit_id(&ret.id);
    }

    fn visit_expr(&mut self, expr: &Expr<'ast, Self::Ast>) {
        use Expr::*;
        match expr {
            Tensor(n) => self.visit_tensor(n),
            Binary(n) => self.visit_binary(n),
            Unary(n) => self.visit_unary(n),
            Array(n) => self.visit_array(n),
            Sel(n) => self.visit_sel(n),
            Id(n) => self.visit_id(n),
            Bool(n) => self.visit_bool(n),
            U32(n) => self.visit_u32(n),
        }
    }

    fn visit_tensor(&mut self, tensor: &Tensor<'ast, Self::Ast>) {
        self.write("{\n");

        self.depth += 1;
        for stmt in &tensor.body {
            self.visit_stmt(stmt);
        }

        self.indent();
        Ast::visit_operand(self, &tensor.ret);
        self.write("\n");

        self.depth -= 1;

        self.indent();
        self.write("| ");
        Ast::visit_operand(self, &tensor.lb);
        self.write(" <= ");
        self.write(&tensor.iv.name);
        self.write(" < ");
        Ast::visit_operand(self, &tensor.ub);
        self.write(" }");
    }

    fn visit_binary(&mut self, binary: &Binary<'ast, Self::Ast>) {
        Ast::visit_operand(self, &binary.l);
        self.write(" ");
        self.write(&binary.op.to_string());
        self.write(" ");
        Ast::visit_operand(self, &binary.r);
    }

    fn visit_unary(&mut self, unary: &Unary<'ast, Self::Ast>) {
        self.write(&unary.op.to_string());
        Ast::visit_operand(self, &unary.r);
    }

    fn visit_array(&mut self, array: &Array<'ast, Self::Ast>) {
        self.write("[");
        for v in &array.values {
            Ast::visit_operand(self, v);
            self.write(", ");
        }
        self.write("]");
    }

    fn visit_sel(&mut self, sel: &Sel<'ast, Self::Ast>) {
        Ast::visit_operand(self, &sel.arr);
        self.write("[");
        for idx in &sel.idx {
            Ast::visit_operand(self, idx);
            self.write(",");
        }
        self.write("]");
    }

    fn visit_id(&mut self, id: &Id<'ast, Self::Ast>) {
        match id {
            Id::Arg(i) => self.write(&self.args[*i].name),
            Id::Var(v) => self.write(&<Ast as AstConfig>::var_name(v)),
        }
    }

    fn visit_bool(&mut self, value: &bool) {
        self.write(&value.to_string());
    }

    fn visit_u32(&mut self, value: &u32) {
        self.write(&value.to_string());
    }

    fn visit_type(&mut self, ty: &Type) {
        let ty_str = match ty.ty {
            BaseType::U32 => "u32",
            BaseType::Bool => "bool",
        };
        self.write(ty_str);

        let shp_str = match &ty.shp {
            Shape::Scalar => "",
            Shape::Vector(n) => &format!("[{}]", n),
        };
        self.write(shp_str);
    }
}
