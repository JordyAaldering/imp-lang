use crate::{ast::*, Visit};

pub struct CompileC {
    output: String,
    arg_names: Vec<String>,
    expr_stack: Vec<String>,
    tensor_target: Option<(String, Type)>,
    indent: usize,
}

impl CompileC {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            arg_names: Vec::new(),
            expr_stack: Vec::new(),
            tensor_target: None,
            indent: 0,
        }
    }

    pub fn finish(self) -> String {
        self.output
    }

    fn push_line(&mut self, line: &str) {
        self.output.push_str(&"    ".repeat(self.indent));
        self.output.push_str(line);
        self.output.push('\n');
    }

    fn render_expr<'ast>(&mut self, expr: &Expr<'ast, TypedAst>) -> String {
        self.visit_expr(expr);
        self.expr_stack.pop().expect("expression stack underflow")
    }
}

impl<'ast> Visit<'ast> for CompileC {
    type Ast = TypedAst;

    fn visit_program(&mut self, program: &Program<'ast, TypedAst>) {
        self.output.push_str("#include <stdlib.h>\n");
        self.output.push_str("#include <stdbool.h>\n");
        self.output.push_str("#include <stdint.h>\n");
        self.output.push('\n');
        self.output.push_str("typedef struct {\n");
        self.output.push_str("    size_t len;\n");
        self.output.push_str("    size_t dim;\n");
        self.output.push_str("    size_t *shp;\n");
        self.output.push_str("    uint32_t *data;\n");
        self.output.push_str("} ImpArrayu32Raw;\n\n");

        for fundef in &program.fundefs {
            self.visit_fundef(fundef);
            self.output.push('\n');
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, TypedAst>) {
        self.arg_names = fundef.args.iter().map(|arg| arg.name.clone()).collect();
        let args: Vec<String> = fundef.args.iter()
            .map(|arg| format!("{} {}", full_ctype(&arg.ty), arg.name))
            .collect();

        self.push_line(&format!(
            "{} IMP_{}({}) {{",
            full_ctype(&fundef.ret_type), fundef.name, args.join(", ")
        ));

        self.indent += 1;
        for stmt in &fundef.body {
            self.visit_stmt(stmt);
        }
        self.indent -= 1;

        self.push_line("}");
    }

    fn visit_assign(&mut self, assign: &Assign<'ast, Self::Ast>) {
        if let Expr::Tensor(tensor) = assign.expr {
            self.tensor_target = Some((assign.lvis.name.clone(), assign.lvis.ty.clone()));
            self.visit_tensor(tensor);
            self.tensor_target = None;
            return;
        }

        let rhs = self.render_expr(assign.expr);
        self.push_line(&format!("{} {} = {};", full_ctype(&assign.lvis.ty), assign.lvis.name, rhs));
    }

    fn visit_return(&mut self, ret: &Return<'ast, Self::Ast>) {
        let name = self.render_expr(&Expr::Id(ret.id.clone()));
        self.push_line(&format!("return {};", name));
    }

    fn visit_tensor(&mut self, tensor: &Tensor<'ast, Self::Ast>) {
        let (target_name, target_ty) = self.tensor_target.clone().expect("tensor target must be set");
        let data_name = format!("{}_data", target_name);
        let shp_name = format!("{}_shp", target_name);
        let len_name = format!("{}_len", target_name);

        let lb = self.render_expr(&Expr::Id(tensor.lb.clone()));
        let ub = self.render_expr(&Expr::Id(tensor.ub.clone()));
        let iv = &tensor.iv.name;
        let base = base_ctype(&target_ty);

        self.push_line(&format!("size_t {} = (size_t)({} - {});", len_name, ub, lb));
        self.push_line(&format!("{} *{} = ({} *)malloc({} * sizeof({}));", base, data_name, base, len_name, base));
        self.push_line(&format!(
            "for (size_t {} = {}; {} < {}; {} += 1) {{",
            iv, lb, iv, ub, iv
        ));

        self.indent += 1;
        for stmt in &tensor.body {
            self.visit_stmt(stmt);
        }
        let ret = self.render_expr(&Expr::Id(tensor.ret.clone()));
        self.push_line(&format!("{}[{} - {}] = {};", data_name, iv, lb, ret));
        self.indent -= 1;

        self.push_line("}");
        self.push_line(&format!("size_t *{} = (size_t *)malloc(sizeof(size_t));", shp_name));
        self.push_line(&format!("{}[0] = {};", shp_name, len_name));
        self.push_line(&format!(
            "ImpArrayu32Raw {} = (ImpArrayu32Raw) {{ .len = {}, .shp = {}, .dim = 1, .data = {} }};",
            target_name, len_name, shp_name, data_name
        ));
    }

    fn visit_binary(&mut self, binary: &Binary<'ast, Self::Ast>) {
        let l = self.render_expr(&Expr::Id(binary.l.clone()));
        let r = self.render_expr(&Expr::Id(binary.r.clone()));
        self.expr_stack.push(format!("{} {} {}", l, binary.op, r));
    }

    fn visit_unary(&mut self, unary: &Unary<'ast, Self::Ast>) {
        let r = self.render_expr(&Expr::Id(unary.r.clone()));
        self.expr_stack.push(format!("{}{}", unary.op, r));
    }

    fn visit_id(&mut self, id: &Id<'ast, Self::Ast>) {
        let name = match id {
            Id::Arg(i) => self.arg_names[*i].clone(),
            Id::Var(lvis) => lvis.name.clone(),
        };
        self.expr_stack.push(name);
    }

    fn visit_bool(&mut self, v: &bool) {
        self.expr_stack.push(if *v { "true".to_owned() } else { "false".to_owned() });
    }

    fn visit_u32(&mut self, v: &u32) {
        self.expr_stack.push(v.to_string());
    }
}

fn base_ctype(ty: &Type) -> &'static str {
    match ty.ty {
        BaseType::U32 => "uint32_t",
        BaseType::Bool => "bool",
    }
}

fn full_ctype(ty: &Type) -> String {
    match &ty.shp {
        Shape::Scalar => base_ctype(ty).to_owned(),
        Shape::Vector(_) => "ImpArrayu32Raw".to_owned(),
    }
}
