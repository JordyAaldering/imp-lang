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
        for fundef in program.functions.values() {
            self.visit_fundef(fundef);
            self.write("\n");
        }

        for fundef in program.generic_functions.values() {
            self.write(&format!("fn {}<{}>(", fundef.name, fundef.type_param));
            for arg in &fundef.args {
                self.write_poly_type(&arg.ty);
                self.write(&format!(" {}, ", arg.name));
            }
            self.write(") -> ");
            self.write_poly_type(&fundef.ret_type);
            if !fundef.where_bounds.is_empty() {
                self.write("\nwhere\n");
                for bound in &fundef.where_bounds {
                    self.write("    ");
                    self.write(&bound.ty_name);
                    self.write(": ");
                    self.write(&bound.trait_name);
                    self.write("\n");
                }
            }
            self.write("{\n");
            self.depth += 1;
            for stmt in &fundef.body {
                self.visit_stmt(stmt);
            }
            self.depth -= 1;
            self.write("}\n");
        }

        for trait_def in program.traits.values() {
            self.write(&format!("trait {}<{}> {{\n", trait_def.name, trait_def.param));
            self.depth += 1;
            for method in &trait_def.methods {
                self.indent();
                self.write("fn ");
                self.write(&method.name);
                self.write("(");
                for arg in &method.args {
                    self.write_poly_type(&arg.ty);
                    self.write(&format!(" {}, ", arg.name));
                }
                self.write(") -> ");
                self.write_poly_type(&method.ret_type);
                self.write(";\n");
            }
            self.depth -= 1;
            self.write("}\n");
        }

        for impl_def in &program.impls {
            self.write(&format!("impl {}<", impl_def.trait_name));
            self.write_poly_type(&impl_def.for_type);
            self.write(">\n");
            if !impl_def.where_bounds.is_empty() {
                self.write("    where ");
                for (i, bound) in impl_def.where_bounds.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&bound.ty_name);
                    self.write(": ");
                    self.write(&bound.trait_name);
                }
                self.write("\n");
            }
            self.write("{\n");
            self.depth += 1;
            for method in &impl_def.methods {
                self.indent();
                self.write("fn ");
                self.write(&method.name);
                self.write("(");
                for arg in &method.args {
                    self.write_poly_type(&arg.ty);
                    self.write(&format!(" {}, ", arg.name));
                }
                self.write(") -> ");
                self.write_poly_type(&method.ret_type);
                self.write(" { /* body omitted during early trait refactor */ }\n");
            }
            self.depth -= 1;
            self.write("}\n");
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
            Call(n) => self.visit_call(n),
            PrfCall(n) => self.visit_prf_call(n),
            Tensor(n) => self.visit_tensor(n),
            Array(n) => self.visit_array(n),
            Sel(n) => self.visit_sel(n),
            Id(n) => self.visit_id(n),
            Bool(n) => self.visit_bool(n),
            U32(n) => self.visit_u32(n),
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
        self.write(&prf_call.id.to_string());
        self.write("(");
        for arg in &prf_call.args {
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
        for v in &array.values {
            Ast::visit_operand(self, v);
            self.write(", ");
        }
        self.write("]");
    }

    fn visit_sel(&mut self, sel: &Sel<'ast, Self::Ast>) {
        Ast::visit_operand(self, &sel.arr);
        self.write("[");
        Ast::visit_operand(self, &sel.idx);
        self.write("]");
    }

    fn visit_id(&mut self, id: &Id<'ast, Self::Ast>) {
        match id {
            Id::Arg(i) => self.write(&self.args[*i].name),
            Id::Var(v) => self.write(&<Ast as AstConfig>::var_name(v)),
            Id::Dim(i) => self.write(&format!("{}.dim", self.args[*i].name)),
            Id::Shp(i) => self.write(&format!("{}.shp", self.args[*i].name)),
            Id::DimAt(i, k) => self.write(&format!("{}.shp[{k}]", self.args[*i].name)),
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
            BaseType::Usize => "usize",
            BaseType::Bool => "bool",
        };
        self.write(ty_str);

        match &ty.shape {
            ShapePattern::Scalar => {}
            ShapePattern::Any => self.write("[*]"),
            ShapePattern::Axes(axes) => {
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
    fn write_poly_type(&mut self, ty: &PolyType) {
        self.write(&ty.head);
        if let Some(shape) = &ty.shape {
            match shape {
                ShapePattern::Scalar => {}
                ShapePattern::Any => self.write("[*]"),
                ShapePattern::Axes(axes) => {
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
                        self.write(",");
                    }
                    self.write("]");
                }
            }
        }
    }
}
