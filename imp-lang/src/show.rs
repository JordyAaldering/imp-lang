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
            self.write(";\n");
        }
        self.depth -= 1;

        self.visit_body(&fundef.body);
        self.write("\n");
        self.indent();
        self.write("}\n");
    }

    fn visit_farg(&mut self, arg: &Farg) {
        self.visit_type(&arg.ty);
        self.write(&format!(" {}, ", arg.id));
    }

    fn visit_body(&mut self, body: &Body<'ast, Self::Ast>) {
        self.depth += 1;

        for stmt in &body.stmts {
            self.visit_stmt(stmt);
        }

        self.indent();
        Self::Ast::visit_operand(self, &body.ret);

        self.depth -= 1;
    }

    fn visit_assign(&mut self, assign: &Assign<'ast, Self::Ast>) {
        self.indent();
        self.write(&assign.lhs.name);
        self.write(" = ");
        self.visit_expr(assign.expr);
        self.write(";\n");
    }

    fn visit_cond(&mut self, cond: &Cond<'ast, Self::Ast>) {
        self.write("if ");
        Self::Ast::visit_operand(self, &cond.cond);
        self.write(" {");
        Self::Ast::visit_operand(self, &cond.then_branch);
        self.write("} else {");
        Self::Ast::visit_operand(self, &cond.else_branch);
        self.write("}");
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

    fn visit_fold(&mut self, fold: &Fold<'ast, Self::Ast>) {
        self.write("@fold(");
        Ast::visit_operand(self, &fold.neutral);
        self.write(", ");
        match &fold.foldfun {
            FoldFun::Name(id) => self.write(&Self::Ast::dispatch_name(id)),
            FoldFun::Apply { id, args } => {
                self.write(&Self::Ast::dispatch_name(id));
                self.write("(");
                for arg in args {
                    match arg {
                        FoldFunArg::Placeholder => self.write("_"),
                        FoldFunArg::Bound(bound) => Ast::visit_operand(self, bound),
                    }
                    self.write(", ");
                }
                self.write(")");
            }
        }
        self.write(", ");
        self.visit_tensor(&fold.selection);
        self.write(")");
    }

    fn visit_tensor(&mut self, tensor: &Tensor<'ast, Self::Ast>) {
        self.write("{\n");
        self.visit_body(&tensor.body);
        self.write(" | ");

        if let Some(lb) = &tensor.lb {
            Ast::visit_operand(self, lb);
            self.write(" <= ");
        }

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
                        AxisPattern::Dim(DimPattern::Var(var)) => self.write(&var),
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
            Bool => "bool",
            I32 => "i32",
            I64 => "i64",
            U32 => "u32",
            U64 => "u64",
            Usize => "usize",
            F32 => "f32",
            F64 => "f64",
            Udf(udf) => udf,
        };
        self.write(ty_str);
    }

}
