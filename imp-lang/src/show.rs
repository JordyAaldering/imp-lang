use crate::ast::*;

pub fn show<'ast, Ast: AstConfig + 'ast>(program: &mut Program<'ast, Ast>) -> String {
    let mut show: Show<'ast, Ast> = Show::new();
    show.trav_program(program);
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

impl<'ast, Ast: AstConfig + 'ast> Traverse<'ast> for Show<'ast, Ast> {
    type Ast = Ast;

    type ExprOut = ();

    const EXPR_DEFAULT: Self::ExprOut = ();

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        self.args = fundef.args.clone();

        self.write(&format!("fn {}(", fundef.name));
        self.trav_fargs(&mut fundef.args);
        self.write(") -> ");
        self.trav_type(&mut fundef.ret_type);
        self.write(" {\n");

        self.depth += 1;

        self.indent();
        self.write("// Variable declarations\n");
        for vardec in fundef.decs.iter_mut() {
            self.trav_vardec(vardec);
        }

        self.indent();
        self.write("// Shape prelude:\n");
        for stmt in &mut fundef.shape_prelude {
            self.trav_assign(stmt);
        }

        self.indent();
        self.write("// Function body:\n");

        self.depth -= 1;

        self.trav_body(&mut fundef.body);
        self.write("\n");
        self.indent();
        self.write("}\n");
    }

    fn trav_vardec(&mut self, vardec: &mut VarInfo<'ast, Self::Ast>) {
        self.indent();
        Self::Ast::trav_type(self, &mut vardec.ty);
        self.write(" ");
        self.write(&vardec.name);
        self.write(";\n");
    }

    fn trav_farg(&mut self, arg: &mut Farg) {
        self.trav_type(&mut arg.ty);
        self.write(&format!(" {}, ", arg.id));
    }

    fn trav_body(&mut self, body: &mut Body<'ast, Self::Ast>) {
        self.depth += 1;

        for stmt in &mut body.stmts {
            self.trav_stmt(stmt);
        }

        self.indent();
        Self::Ast::trav_operand(self, &mut body.ret);

        self.depth -= 1;
    }

    fn trav_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        self.indent();
        self.write(&assign.lhs.name);
        self.write(" = ");
        self.trav_expr(assign.expr);
        self.write(";\n");
    }

    fn trav_cond(&mut self, cond: &mut Cond<'ast, Self::Ast>) {
        self.write("if ");
        Self::Ast::trav_operand(self, &mut cond.cond);
        self.write(" {\n");
        self.trav_body(&mut cond.then_branch);
        self.write("\n");
        self.indent();
        self.write("} else {\n");
        self.trav_body(&mut cond.else_branch);
        self.write("}");
    }

    fn trav_call(&mut self, call: &mut Call<'ast, Self::Ast>) {
        self.write(&Self::Ast::dispatch_name(&call.id));

        self.write("(");
        for arg in &mut call.args {
            Self::Ast::trav_operand(self, arg);
            self.write(", ");
        }

        self.write(")");
    }

    fn trav_prf(&mut self, prf: &mut Prf<'ast, Self::Ast>) {
        self.write(prf.nameof());
        self.write("(");
        for arg in prf.args_mut() {
            Self::Ast::trav_operand(self, arg);
            self.write(", ");
        }
        self.write(")");
    }

    fn trav_fold(&mut self, fold: &mut Fold<'ast, Self::Ast>) {
        self.write("fold(");

        Ast::trav_operand(self, &mut fold.neutral);

        self.write(", ");
        match &mut fold.foldfun {
            FoldFun::Name(id) => self.write(&Self::Ast::dispatch_name(id)),
            FoldFun::Apply { id, args } => {
                self.write(&Self::Ast::dispatch_name(id));
                self.write("(");
                for arg in args {
                    match arg {
                        FoldFunArg::Placeholder => self.write("_"),
                        FoldFunArg::Bound(bound) => Ast::trav_operand(self, bound),
                    }
                    self.write(", ");
                }
                self.write(")");
            }
        }

        self.write(", ");

        self.trav_tensor(&mut fold.selection);

        self.write(")");
    }

    fn trav_tensor(&mut self, tensor: &mut Tensor<'ast, Self::Ast>) {
        self.write("{\n");
        self.trav_body(&mut tensor.body);
        self.write(" | ");

        if let Some(lb) = &mut tensor.lb {
            Ast::trav_operand(self, lb);
            self.write(" <= ");
        }

        self.write(&tensor.iv.name);
        self.write(" < ");
        Ast::trav_operand(self, &mut tensor.ub);
        self.write(" }");
    }

    fn trav_array(&mut self, array: &mut Array<'ast, Self::Ast>) {
        self.write("[");
        for v in &mut array.elems {
            Ast::trav_operand(self, v);
            self.write(", ");
        }
        self.write("]");
    }

    fn trav_id(&mut self, id: &mut Id<'ast, Self::Ast>) {
        match id {
            Id::Arg(i) => self.write(&self.args[*i].id.clone()),
            Id::Var(v) => self.write(&<Ast as AstConfig>::var_name(v)),
        }
    }

    fn trav_const(&mut self, c: &mut Const) {
        use Const::*;
        match c {
            Bool(v) => self.write(&v.to_string()),
            Usize(v) => self.write(&v.to_string()),
            U32(v) => self.write(&v.to_string()),
            U64(v) => self.write(&v.to_string()),
            I32(v) => self.write(&v.to_string()),
            I64(v) => self.write(&v.to_string()),
            F32(v) => self.write(&v.to_string()),
            F64(v) => self.write(&v.to_string()),
        }
    }

    fn trav_type(&mut self, ty: &mut Type) {
        use BaseType::*;
        let ty_str = match &ty.ty {
            Bool => "bool",
            Usize => "usize",
            U32 => "u32",
            U64 => "u64",
            I32 => "i32",
            I64 => "i64",
            F32 => "f32",
            F64 => "f64",
            Udf(udf) => udf,
        };
        self.write(ty_str);

        match &ty.shape {
            TypePattern::Scalar => {}
            TypePattern::Axes(axes) => {
                self.write("[");
                for axis in axes {
                    match axis {
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
