use crate::{ast::*, cg::rename_fundefs, Visit};

pub struct CompileC {
    output: String,
    stem: String,
    arg_names: Vec<String>,
    arg_types: Vec<Type>,
    expr_stack: Vec<String>,
    lhs_target: Option<(String, Type)>,
    indent: usize,
}

impl CompileC {
    pub fn new(stem: &str) -> Self {
        Self {
            output: String::new(),
            stem: stem.to_owned(),
            arg_names: Vec::new(),
            arg_types: Vec::new(),
            expr_stack: Vec::new(),
            lhs_target: None,
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

    fn render_expr(&mut self, expr: &Expr<'_, TypedAst>) -> String {
        self.visit_expr(expr);
        self.expr_stack.pop().expect("expression stack underflow")
    }

    fn nameof(&mut self, id: &Id<'_, TypedAst>) -> String {
        match id {
            Id::Arg(i) => self.arg_names[*i].clone(),
            Id::Var(v) => v.name.clone(),
        }
    }

    fn id_type<'a>(&'a self, id: &'a Id<'_, TypedAst>) -> &'a Type {
        match id {
            Id::Arg(i) => &self.arg_types[*i],
            Id::Var(v) => &v.ty,
        }
    }
}

impl<'ast> Visit<'ast> for CompileC {
    type Ast = TypedAst;

    fn visit_program(&mut self, program: &Program<'ast, TypedAst>) {
        self.output.push_str(&format!("#include \"{}.h\"\n\n", self.stem));
        self.output.push_str("__attribute__((unused)) static size_t imp_flat_index_u32(ImpArrayRaw arr, ImpArrayRaw idx) {\n");
        self.output.push_str("    size_t flat = 0;\n");
        self.output.push_str("    uint32_t *idx_data = (uint32_t *)idx.data;\n");
        self.output.push_str("    for (size_t d = 0; d < idx.len; d += 1) {\n");
        self.output.push_str("        flat = flat * arr.shp[d] + (size_t)idx_data[d];\n");
        self.output.push_str("    }\n");
        self.output.push_str("    return flat;\n");
        self.output.push_str("}\n\n");
        self.output.push_str("__attribute__((unused)) static size_t imp_flat_index_usize(ImpArrayRaw arr, ImpArrayRaw idx) {\n");
        self.output.push_str("    size_t flat = 0;\n");
        self.output.push_str("    size_t *idx_data = (size_t *)idx.data;\n");
        self.output.push_str("    for (size_t d = 0; d < idx.len; d += 1) {\n");
        self.output.push_str("        flat = flat * arr.shp[d] + idx_data[d];\n");
        self.output.push_str("    }\n");
        self.output.push_str("    return flat;\n");
        self.output.push_str("}\n\n");

        for wrapper in program.fundefs.values() {
            for fundef in &wrapper.overloads {
                self.visit_fundef(fundef);
                self.output.push('\n');
            }
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, TypedAst>) {
        self.arg_names = fundef.args.iter().map(|arg| arg.name.clone()).collect();
        self.arg_types = fundef.args.iter().map(|arg| arg.ty.clone()).collect();
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
            self.lhs_target = Some((assign.lvis.name.clone(), assign.lvis.ty.clone()));
            self.visit_tensor(tensor);
            self.lhs_target = None;
            return;
        }

        if let Expr::Array(array) = assign.expr {
            self.lhs_target = Some((assign.lvis.name.clone(), assign.lvis.ty.clone()));
            self.visit_array(array);
            self.lhs_target = None;
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
        let (target_name, target_ty) = self.lhs_target.clone().expect("tensor target must be set");
        let base = base_ctype(&target_ty);
        let iv_name = tensor.iv.name.clone();

        // ── Backward-compat path: scalar iv (old syntax where lb/ub are plain scalars) ──
        if tensor.iv.ty.is_scalar() {
            let data_name = format!("{target_name}_data");
            let shp_name  = format!("{target_name}_shp");
            let len_name  = format!("{target_name}_len");
            let lb = self.render_expr(&Expr::Id(tensor.lb.clone()));
            let ub = self.render_expr(&Expr::Id(tensor.ub.clone()));
            self.push_line(&format!("size_t {len_name} = (size_t)({ub});"));
            self.push_line(&format!("{base} *{data_name} = ({base} *)malloc({len_name} * sizeof({base}));"));
            self.push_line(&format!("for (size_t {iv_name} = (size_t)({lb}); {iv_name} < (size_t)({ub}); {iv_name} += 1) {{"));
            self.indent += 1;
            for stmt in &tensor.body { self.visit_stmt(stmt); }
            let ret = self.render_expr(&Expr::Id(tensor.ret.clone()));
            self.push_line(&format!("{data_name}[{iv_name}] = {ret};"));
            self.indent -= 1;
            self.push_line("}");
            self.push_line(&format!("size_t *{shp_name} = (size_t *)malloc(sizeof(size_t));"));
            self.push_line(&format!("{shp_name}[0] = {len_name};"));
            self.push_line(&format!(
                "ImpArrayRaw {target_name} = (ImpArrayRaw) {{ .len = {len_name}, .shp = {shp_name}, .dim = 1, .data = (void *){data_name} }};"
            ));
            return;
        }

        // ── New path: vector iv, lb, ub are ImpArrayRaw vectors ──
        let rank = tensor.iv.ty.rank()
            .expect("tensor iv must have a statically-known rank for C codegen") as usize;

        let lb_name = self.nameof(&tensor.lb);
        let ub_name = self.nameof(&tensor.ub);
        // Determine the element type stored inside lb/ub (for correct pointer cast).
        let lb_elem = elem_ctype_of_id(&tensor.lb);
        let ub_elem = elem_ctype_of_id(&tensor.ub);

        // Extract scalar lower/upper bound per dimension.
        for d in 0..rank {
            self.push_line(&format!(
                "size_t {iv_name}_lb{d} = (size_t)(({lb_elem}*){lb_name}.data)[{d}];"
            ));
            self.push_line(&format!(
                "size_t {iv_name}_ub{d} = (size_t)(({ub_elem}*){ub_name}.data)[{d}];"
            ));
        }

        // Total element count in the result (product of extents).
        let len_name  = format!("{target_name}_len");
        let data_name = format!("{target_name}_data");
        let shp_name  = format!("{target_name}_shp");
        let extents: Vec<String> = (0..rank)
            .map(|d| format!("({iv_name}_ub{d} - {iv_name}_lb{d})"))
            .collect();
        let total_len = if extents.is_empty() { "1".to_owned() } else { extents.join(" * ") };
        self.push_line(&format!("size_t {len_name} = {total_len};"));
        self.push_line(&format!("{base} *{data_name} = ({base} *)malloc({len_name} * sizeof({base}));"));

        // Heap-allocate the result shape array.
        self.push_line(&format!("size_t *{shp_name} = (size_t *)malloc({rank} * sizeof(size_t));"));
        for d in 0..rank {
            self.push_line(&format!("{shp_name}[{d}] = {iv_name}_ub{d} - {iv_name}_lb{d};"));
        }

        // Generate k nested for-loops.
        for d in 0..rank {
            self.push_line(&format!(
                "for (size_t {iv_name}_{d} = {iv_name}_lb{d}; {iv_name}_{d} < {iv_name}_ub{d}; {iv_name}_{d} += 1) {{"
            ));
            self.indent += 1;
        }

        // Build iv as a stack-allocated ImpArrayRaw so that iv[i] selections work.
        let iv_elem = base_ctype(&tensor.iv.ty);
        let iv_components: Vec<String> = (0..rank)
            .map(|d| format!("({iv_elem}){iv_name}_{d}"))
            .collect();
        self.push_line(&format!(
            "{iv_elem} {iv_name}_data[{rank}] = {{ {} }};",
            iv_components.join(", ")
        ));
        self.push_line(&format!("size_t {iv_name}_shp_arr[1] = {{ {rank} }};"));
        self.push_line(&format!(
            "ImpArrayRaw {iv_name} = (ImpArrayRaw) {{ .len = {rank}, .shp = {iv_name}_shp_arr, .dim = 1, .data = (void *){iv_name}_data }};"
        ));

        // Row-major flat index: Σ (iv_d - lb_d) * stride_d
        let flat_terms: Vec<String> = (0..rank).map(|d| {
            let stride: Vec<String> = (d + 1..rank)
                .map(|j| format!("({iv_name}_ub{j} - {iv_name}_lb{j})"))
                .collect();
            let stride_expr = if stride.is_empty() { "1".to_owned() } else { stride.join(" * ") };
            format!("({iv_name}_{d} - {iv_name}_lb{d}) * {stride_expr}")
        }).collect();
        let flat_expr = if flat_terms.is_empty() { "0".to_owned() } else { flat_terms.join(" + ") };
        self.push_line(&format!("size_t {iv_name}_flat = {flat_expr};"));

        // Body statements.
        for stmt in &tensor.body {
            self.visit_stmt(stmt);
        }

        // Store element into the flat result buffer.
        let ret = self.render_expr(&Expr::Id(tensor.ret.clone()));
        self.push_line(&format!("{data_name}[{iv_name}_flat] = {ret};"));

        // Close nested loops.
        for _ in 0..rank {
            self.indent -= 1;
            self.push_line("}");
        }

        self.push_line(&format!(
            "ImpArrayRaw {target_name} = (ImpArrayRaw) {{ .len = {len_name}, .shp = {shp_name}, .dim = {rank}, .data = (void *){data_name} }};"
        ));
    }

    fn visit_binary(&mut self, binary: &Binary<'ast, Self::Ast>) {
        if matches!(self.id_type(&binary.l).shape, ShapePattern::Any)
            || matches!(self.id_type(&binary.r).shape, ShapePattern::Any)
        {
            panic!("dynamic union values are not yet supported in binary ops during C codegen");
        }
        let l = self.render_expr(&Expr::Id(binary.l.clone()));
        let r = self.render_expr(&Expr::Id(binary.r.clone()));
        self.expr_stack.push(format!("{} {} {}", l, binary.op, r));
    }

    fn visit_unary(&mut self, unary: &Unary<'ast, Self::Ast>) {
        if matches!(self.id_type(&unary.r).shape, ShapePattern::Any) {
            panic!("dynamic union values are not yet supported in unary ops during C codegen");
        }
        let r = self.render_expr(&Expr::Id(unary.r.clone()));
        self.expr_stack.push(format!("{}{}", unary.op, r));
    }

    fn visit_array(&mut self, array: &Array<'ast, Self::Ast>) {
        let (target_name, target_ty) = self.lhs_target.clone().expect("array target must be set");
        let data_name = format!("{}_data", target_name);
        let shp_name = format!("{}_shp", target_name);
        let len_name = format!("{}_len", target_name);
        let base = base_ctype(&target_ty);

        self.push_line(&format!("size_t {} = {};", len_name, array.values.len()));
        self.push_line(&format!("{} *{} = ({} *)malloc({} * sizeof({}));", base, data_name, base, len_name, base));

        for (i, value) in array.values.iter().enumerate() {
            let rendered = self.render_expr(&Expr::Id(value.clone()));
            self.push_line(&format!("{}[{}] = {};", data_name, i, rendered));
        }

        self.push_line(&format!("size_t *{} = (size_t *)malloc(sizeof(size_t));", shp_name));
        self.push_line(&format!("{}[0] = {};", shp_name, len_name));
        self.push_line(&format!(
            "ImpArrayRaw {} = (ImpArrayRaw) {{ .len = {}, .shp = {}, .dim = 1, .data = (void *){} }};",
            target_name, len_name, shp_name, data_name
        ));
    }

    fn visit_sel(&mut self, sel: &Sel<'ast, Self::Ast>) {
        if matches!(self.id_type(&sel.arr).shape, ShapePattern::Any)
            || matches!(self.id_type(&sel.idx).shape, ShapePattern::Any)
        {
            panic!("dynamic union values are not yet supported in selection during C codegen");
        }
        let arr = self.nameof(&sel.arr);
        let idx = self.nameof(&sel.idx);
        let elem_base = elem_ctype_of_id(&sel.arr);
        let flat_fn = flat_index_fn_of_id(&sel.idx);

        self.expr_stack.push(format!("(({elem_base} *){arr}.data)[{flat_fn}({arr}, {idx})]"));
    }

    fn visit_call(&mut self, call: &Call<'ast, TypedAst>) {
        let base_name = TypedAst::dispatch_name(&call.id);
        let arg_types: Vec<Type> = call.args.iter().map(|id| match id {
            Id::Arg(i) => self.arg_types[*i].clone(),
            Id::Var(v) => v.ty.clone(),
        }).collect();
        let name = rename_fundefs::mangle_call_name(&base_name, &arg_types);
        let args: Vec<String> = call.args.iter()
            .map(|arg| self.nameof(arg))
            .collect();
        self.expr_stack.push(format!("IMP_{}({})", name, args.join(", ")));
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
        BaseType::Usize => "size_t",
        BaseType::Bool => "bool",
    }
}

fn full_ctype(ty: &Type) -> String {
    if matches!(ty.shape, ShapePattern::Any) {
        return match ty.ty {
            BaseType::U32 => "ImpDynU32".to_owned(),
            BaseType::Usize => "ImpDynUsize".to_owned(),
            BaseType::Bool => "ImpDynBool".to_owned(),
        };
    }

    if ty.is_array() {
        "ImpArrayRaw".to_owned()
    } else {
        base_ctype(ty).to_owned()
    }
}

/// The C element type for the data pointer stored inside an ImpArrayRaw id.
fn elem_ctype_of_id(id: &Id<'_, TypedAst>) -> &'static str {
    match id {
        Id::Var(v) => base_ctype(&v.ty),
        Id::Arg(_) => "uint32_t",  // args used directly as array bounds are uncommon
    }
}

fn flat_index_fn_of_id(id: &Id<'_, TypedAst>) -> &'static str {
    match id_base_type(id) {
        BaseType::U32 => "imp_flat_index_u32",
        BaseType::Usize => "imp_flat_index_usize",
        BaseType::Bool => panic!("bool cannot be used as an array index vector"),
    }
}

fn id_base_type(id: &Id<'_, TypedAst>) -> BaseType {
    match id {
        Id::Var(v) => v.ty.ty,
        Id::Arg(_) => BaseType::U32,
    }
}
