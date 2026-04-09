use crate::{ast::*, Visit};

pub fn show<'ast, Ast: AstConfig + 'ast>(program: &Program<'ast, Ast>) -> String {
    let mut show: Show<'ast, Ast> = Show::new();
    show.visit_program(program);
    show.output
}

struct Show<'ast, Ast: AstConfig> {
    args: Vec<Farg>,
    depth: usize,
    output: String,
    _phantom: std::marker::PhantomData<&'ast Ast>,
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
        for fundef in program.functions.values() {
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
            self.write(" ");
            self.write(&id.name);
            self.write("/* meta-info here, like AKV constant value knowledge and other relevant info */");
            self.write(";\n");
        }
        for stmt in &fundef.body {
            self.visit_stmt(stmt);
        }
        self.depth -= 1;

        self.write("}");
    }

    fn visit_farg(&mut self, arg: &Farg) {
        self.visit_type(&arg.ty);
        self.write(&format!(" {}, ", arg.id));
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
        self.write(&assign.lhs.name);
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
            Call(n) => self.visit_call(n),
            PrfCall(n) => self.visit_prf_call(n),
            Tensor(n) => self.visit_tensor(n),
            Array(n) => self.visit_array(n),
            Id(n) => self.visit_id(n),
            Const(n) => self.visit_const(n),
        }
    }

    fn visit_call(&mut self, call: &Call<'ast, Self::Ast>) {
        self.write(&Self::Ast::dispatch_name(&call.id));

        self.write("(");
        for arg in &call.args {
            Self::Ast::visit_operand(self, arg);
            self.write(", ");
        }

        self.write(")");
    }

    fn visit_prf_call(&mut self, prf_call: &PrfCall<'ast, Self::Ast>) {
        self.write(prf_call.nameof());
        self.write("(");
        for arg in prf_call.args() {
            Self::Ast::visit_operand(self, arg);
            self.write(", ");
        }
        self.write(")");
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

    fn visit_array(&mut self, array: &Array<'ast, Self::Ast>) {
        self.write("[");
        for v in &array.elems {
            Ast::visit_operand(self, v);
            self.write(", ");
        }
        self.write("]");
    }

    fn visit_id(&mut self, id: &Id<'ast, Self::Ast>) {
        match id {
            Id::Arg(i) => self.write(&self.args[*i].id.clone()),
            Id::Var(v) => self.write(&<Ast as AstConfig>::var_name(v)),
            Id::Dim(i) => self.write(&format!("{}.dim", self.args[*i].id)),
            Id::Shp(i) => self.write(&format!("{}.shp", self.args[*i].id)),
            Id::DimAt(i, k) => self.write(&format!("{}.shp[{k}]", self.args[*i].id)),
        }
    }

    fn visit_const(&mut self, c: &Const) {
        use Const::*;
        match c {
            Bool(v) => self.write(&v.to_string()),
            I32(v) => self.write(&v.to_string()),
            I64(v) => self.write(&v.to_string()),
            U32(v) => self.write(&v.to_string()),
            U64(v) => self.write(&v.to_string()),
            Usize(v) => self.write(&v.to_string()),
            F32(v) => self.write(&v.to_string()),
            F64(v) => self.write(&v.to_string()),
        }
    }

    fn visit_type(&mut self, ty: &Type) {
        self.write_basetype(&ty.ty);

        match &ty.shape {
            TypePattern::Scalar => {}
            TypePattern::Any => self.write("[*]"),
            TypePattern::Axes(axes) => {
                self.write("[");
                for axis in axes {
                    match axis {
                        AxisPattern::Dim(DimPattern::Any) => self.write("_"),
                        AxisPattern::Dim(DimPattern::Known(n)) => self.write(&n.to_string()),
                        AxisPattern::Dim(DimPattern::Var(var)) => self.write(&var.name),
                        AxisPattern::Rank(capture) => {
                            self.write(&capture.dim_name);
                            self.write(":");
                            self.write(&capture.shp_name);
                        }
                    }
                    self.write(",")
                }
                self.write("]");
            }
        }
    }
}

impl<'ast, Ast: AstConfig> Show<'ast, Ast> {
    fn write_basetype(&mut self, ty: &BaseType) {
        use BaseType::*;
        let ty_str = match ty {
            I32 => "i32",
            I64 => "i64",
            U32 => "u32",
            U64 => "u64",
            Usize => "usize",
            F32 => "f32",
            F64 => "f64",
            Bool => "bool",
            Udf(udf) => udf,
        };
        self.write(ty_str);
    }

}
