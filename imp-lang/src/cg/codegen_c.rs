use crate::{ast::*, cg::{mono, rename_fundefs}, Visit};
use std::collections::HashSet;

pub struct CompileC {
    output: String,
    stem: String,
    arg_names: Vec<String>,
    arg_types: Vec<Type>,
    expr_stack: Vec<String>,
    lhs_target: Option<(String, Type)>,
    indent: usize,
    shp_uid: usize,
    tensor_uid: usize,
    impls: Vec<ImplDef>,
    emitted_trait_shims: HashSet<String>,
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
            shp_uid: 0,
            tensor_uid: 0,
            impls: Vec::new(),
            emitted_trait_shims: HashSet::new(),
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
            Id::Dim(i) => format!("{}.dim", self.arg_names[*i]),
            Id::DimAt(i, k) => format!("{}.shp[{k}]", self.arg_names[*i]),
            Id::Shp(_) => panic!("nameof called on Id::Shp — use render_expr instead"),
        }
    }

    fn id_is_any(&self, id: &Id<'_, TypedAst>) -> bool {
        match id {
            Id::Arg(i) => matches!(self.arg_types[*i].shape, ShapePattern::Any),
            Id::Var(v) => matches!(v.ty.shape, ShapePattern::Any),
            Id::Dim(_) | Id::Shp(_) | Id::DimAt(_, _) => false,
        }
    }

    fn id_type(&self, id: &Id<'_, TypedAst>) -> Type {
        match id {
            Id::Arg(i) => self.arg_types[*i].clone(),
            Id::Var(v) => v.ty.clone(),
            Id::Dim(_) | Id::DimAt(_, _) => Type::scalar(BaseType::Usize),
            Id::Shp(_) => Type {
                ty: BaseType::Usize,
                shape: ShapePattern::Axes(vec![AxisPattern::Dim(DimPattern::Any)]),
                knowledge: TypeKnowledge::AKD,
            },
        }
    }

    fn operator_ret_type(&self, method_name: &str, arg_types: &[Type]) -> Type {
        match method_name {
            "==" | "!=" | "<" | "<=" | ">" | ">=" | "!" => Type::scalar(BaseType::Bool),
            "+" | "-" | "*" | "/" => arg_types.first().cloned().unwrap_or_else(|| Type::scalar(BaseType::U32)),
            _ => arg_types.first().cloned().unwrap_or_else(|| Type::scalar(BaseType::U32)),
        }
    }

    fn trait_shim_name(&self, trait_name: &str, method_name: &str, arg_types: &[Type]) -> String {
        mono::trait_shim_name(trait_name, method_name, arg_types)
    }

    fn has_impl_for_call(&self, trait_name: &str, method_name: &str, arg_types: &[Type]) -> bool {
        mono::has_impl_for_call(&self.impls, trait_name, method_name, arg_types)
    }

    fn emit_trait_shim(&mut self, trait_name: &str, method_name: &str, arg_types: &[Type]) {
        let shim_name = self.trait_shim_name(trait_name, method_name, arg_types);
        if self.emitted_trait_shims.contains(&shim_name) {
            return;
        }
        if !self.has_impl_for_call(trait_name, method_name, arg_types) {
            panic!("missing impl for {}::{} with arg types {:?}", trait_name, method_name, arg_types);
        }

        self.emitted_trait_shims.insert(shim_name.clone());

        let ret_type = self.operator_ret_type(method_name, arg_types);
        let args_decl = arg_types.iter().enumerate()
            .map(|(i, ty)| format!("{} a{}", full_ctype(ty), i))
            .collect::<Vec<_>>()
            .join(", ");
        self.push_line(&format!("static {} IMP_{}({}) {{", full_ctype(&ret_type), shim_name, args_decl));
        self.indent += 1;
        let binary_array = arg_types.len() == 2 && arg_types[0].is_array() && arg_types[1].is_array();
        if binary_array {
            let op = match method_name {
                "+" => "+",
                "-" => "-",
                "*" => "*",
                "/" => "/",
                _ => panic!("unsupported array trait dispatch operator {}", method_name),
            };
            let elem = base_ctype(&arg_types[0]);
            self.push_line("size_t len = a0.len;");
            self.push_line(&format!("size_t *shp = (size_t *)malloc(a0.dim * sizeof(size_t));"));
            self.push_line("for (size_t i = 0; i < a0.dim; i += 1) { shp[i] = a0.shp[i]; }");
            self.push_line(&format!("{} *data = ({})malloc(len * sizeof({}));", format!("{elem}"), format!("{elem} *"), elem));
            self.push_line(&format!("for (size_t i = 0; i < len; i += 1) {{ data[i] = (({elem}*)a0.data)[i] {op} (({elem}*)a1.data)[i]; }}"));
            self.push_line("ImpArrayRaw out = (ImpArrayRaw) { .len = len, .dim = a0.dim, .shp = shp, .data = (void *)data };");
            self.push_line("return out;");
        } else {
            let expr = match (method_name, arg_types.len()) {
                ("+", 2) => "a0 + a1".to_owned(),
                ("-", 2) => "a0 - a1".to_owned(),
                ("*", 2) => "a0 * a1".to_owned(),
                ("/", 2) => "a0 / a1".to_owned(),
                ("==", 2) => "a0 == a1".to_owned(),
                ("!=", 2) => "a0 != a1".to_owned(),
                ("<", 2) => "a0 < a1".to_owned(),
                ("<=", 2) => "a0 <= a1".to_owned(),
                (">", 2) => "a0 > a1".to_owned(),
                (">=", 2) => "a0 >= a1".to_owned(),
                ("-", 1) => "-a0".to_owned(),
                ("!", 1) => "!a0".to_owned(),
                _ => panic!("unsupported trait dispatch operator {} / {} args", method_name, arg_types.len()),
            };
            self.push_line(&format!("return {};", expr));
        }
        self.indent -= 1;
        self.push_line("}");
        self.push_line("");
    }

    fn emit_trait_shims_for_program<'ast>(&mut self, program: &Program<'ast, TypedAst>) {
        for fundef in program.functions.values() {
            for stmt in &fundef.body {
                self.emit_trait_shims_for_stmt(stmt, &fundef.args);
            }
        }
    }

    fn emit_trait_shims_for_stmt<'ast>(&mut self, stmt: &Stmt<'ast, TypedAst>, args: &[&'ast Farg]) {
        match stmt {
            Stmt::Assign(a) => self.emit_trait_shims_for_expr(a.expr, args),
            Stmt::Return(r) => self.emit_trait_shims_for_id(&r.id, args),
        }
    }

    fn emit_trait_shims_for_expr<'ast>(&mut self, expr: &Expr<'ast, TypedAst>, args: &[&'ast Farg]) {
        match expr {
            Expr::Call(call) => {
                for id in &call.args {
                    self.emit_trait_shims_for_id(id, args);
                }
                if let CallTarget::TraitMethod { trait_name, method_name } = &call.id {
                    let arg_types = call.args.iter().map(|id| type_of_id_in_context(id, args)).collect::<Vec<_>>();
                    self.emit_trait_shim(trait_name, method_name, &arg_types);
                }
            }
            Expr::PrfCall(prf) => {
                for id in &prf.args {
                    self.emit_trait_shims_for_id(id, args);
                }
            }
            Expr::Tensor(t) => {
                self.emit_trait_shims_for_id(&t.lb, args);
                self.emit_trait_shims_for_id(&t.ub, args);
                for stmt in &t.body {
                    self.emit_trait_shims_for_stmt(stmt, args);
                }
                self.emit_trait_shims_for_id(&t.ret, args);
            }
            Expr::Array(a) => {
                for id in &a.values {
                    self.emit_trait_shims_for_id(id, args);
                }
            }
            Expr::Id(id) => self.emit_trait_shims_for_id(id, args),
            Expr::Bool(_) | Expr::U32(_) => {}
        }
    }

    fn emit_trait_shims_for_id<'ast>(&mut self, _id: &Id<'ast, TypedAst>, _args: &[&'ast Farg]) {}
}

impl<'ast> Visit<'ast> for CompileC {
    type Ast = TypedAst;

    fn visit_program(&mut self, program: &Program<'ast, TypedAst>) {
        self.impls = program.impls.clone();
        self.emitted_trait_shims.clear();
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

        self.emit_trait_shims_for_program(program);

        for fundef in program.functions.values() {
            self.visit_fundef(fundef);
            self.output.push('\n');
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
        let name = self.render_expr(&Expr::Id(ret.id));
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
            let lb = self.render_expr(&Expr::Id(tensor.lb));
            let ub = self.render_expr(&Expr::Id(tensor.ub));
            self.push_line(&format!("size_t {len_name} = (size_t)({ub});"));
            self.push_line(&format!("{base} *{data_name} = ({base} *)malloc({len_name} * sizeof({base}));"));
            self.push_line(&format!("for (size_t {iv_name} = (size_t)({lb}); {iv_name} < (size_t)({ub}); {iv_name} += 1) {{"));
            self.indent += 1;
            for stmt in &tensor.body { self.visit_stmt(stmt); }
            let ret = self.render_expr(&Expr::Id(tensor.ret));
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

        self.tensor_uid += 1;
        let t_uid = self.tensor_uid;

        let lb_name = self.nameof(&tensor.lb);
        let ub_name = self.nameof(&tensor.ub);
        // Determine the element type stored inside lb/ub (for correct pointer cast).
        let lb_elem = elem_ctype_of_id(&tensor.lb);
        let ub_elem = elem_ctype_of_id(&tensor.ub);

        // Extract scalar lower/upper bound per dimension.
        for d in 0..rank {
            self.push_line(&format!(
                "size_t {iv_name}_lb{d}_{t_uid} = (size_t)(({lb_elem}*){lb_name}.data)[{d}];"
            ));
            self.push_line(&format!(
                "size_t {iv_name}_ub{d}_{t_uid} = (size_t)(({ub_elem}*){ub_name}.data)[{d}];"
            ));
        }

        // Total element count in the result (product of extents).
        let len_name  = format!("{target_name}_len");
        let data_name = format!("{target_name}_data");
        let shp_name  = format!("{target_name}_shp");
        let extents: Vec<String> = (0..rank)
            .map(|d| format!("({iv_name}_ub{d}_{t_uid} - {iv_name}_lb{d}_{t_uid})"))
            .collect();
        let total_len = if extents.is_empty() { "1".to_owned() } else { extents.join(" * ") };
        self.push_line(&format!("size_t {len_name} = {total_len};"));
        self.push_line(&format!("{base} *{data_name} = ({base} *)malloc({len_name} * sizeof({base}));"));

        // Heap-allocate the result shape array.
        self.push_line(&format!("size_t *{shp_name} = (size_t *)malloc({rank} * sizeof(size_t));"));
        for d in 0..rank {
            self.push_line(&format!("{shp_name}[{d}] = {iv_name}_ub{d}_{t_uid} - {iv_name}_lb{d}_{t_uid};"));
        }

        // Generate k nested for-loops.
        for d in 0..rank {
            self.push_line(&format!(
                "for (size_t {iv_name}_{d}_{t_uid} = {iv_name}_lb{d}_{t_uid}; {iv_name}_{d}_{t_uid} < {iv_name}_ub{d}_{t_uid}; {iv_name}_{d}_{t_uid} += 1) {{"
            ));
            self.indent += 1;
        }

        // Build iv as a stack-allocated ImpArrayRaw so that iv[i] selections work.
        let iv_elem = base_ctype(&tensor.iv.ty);
        let iv_components: Vec<String> = (0..rank)
            .map(|d| format!("({iv_elem}){iv_name}_{d}_{t_uid}"))
            .collect();
        self.push_line(&format!(
            "{iv_elem} {iv_name}_data_{t_uid}[{rank}] = {{ {} }};",
            iv_components.join(", ")
        ));
        self.push_line(&format!("size_t {iv_name}_shp_arr_{t_uid}[1] = {{ {rank} }};"));
        self.push_line(&format!(
            "ImpArrayRaw {iv_name} __attribute__((unused)) = (ImpArrayRaw) {{ .len = {rank}, .shp = {iv_name}_shp_arr_{t_uid}, .dim = 1, .data = (void *){iv_name}_data_{t_uid} }};"
        ));

        // Row-major flat index: Σ (iv_d - lb_d) * stride_d
        let flat_terms: Vec<String> = (0..rank).map(|d| {
            let stride: Vec<String> = (d + 1..rank)
                .map(|j| format!("({iv_name}_ub{j}_{t_uid} - {iv_name}_lb{j}_{t_uid})"))
                .collect();
            let stride_expr = if stride.is_empty() { "1".to_owned() } else { stride.join(" * ") };
            format!("({iv_name}_{d}_{t_uid} - {iv_name}_lb{d}_{t_uid}) * {stride_expr}")
        }).collect();
        let flat_expr = if flat_terms.is_empty() { "0".to_owned() } else { flat_terms.join(" + ") };
        self.push_line(&format!("size_t {iv_name}_flat = {flat_expr};"));

        // Body statements.
        for stmt in &tensor.body {
            self.visit_stmt(stmt);
        }

        // Store element into the flat result buffer.
        let mut ret = self.render_expr(&Expr::Id(tensor.ret));
        if rank == 1 && ret == iv_name {
            ret = format!("(({iv_elem}*){iv_name}.data)[0]");
        }
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

    fn visit_array(&mut self, array: &Array<'ast, Self::Ast>) {
        let (target_name, target_ty) = self.lhs_target.clone().expect("array target must be set");
        let data_name = format!("{}_data", target_name);
        let shp_name = format!("{}_shp", target_name);
        let len_name = format!("{}_len", target_name);
        let base = base_ctype(&target_ty);

        self.push_line(&format!("size_t {} = {};", len_name, array.values.len()));
        self.push_line(&format!("{} *{} = ({} *)malloc({} * sizeof({}));", base, data_name, base, len_name, base));

        for (i, value) in array.values.iter().enumerate() {
            let rendered = self.render_expr(&Expr::Id(*value));
            self.push_line(&format!("{}[{}] = {};", data_name, i, rendered));
        }

        self.push_line(&format!("size_t *{} = (size_t *)malloc(sizeof(size_t));", shp_name));
        self.push_line(&format!("{}[0] = {};", shp_name, len_name));
        self.push_line(&format!(
            "ImpArrayRaw {} = (ImpArrayRaw) {{ .len = {}, .shp = {}, .dim = 1, .data = (void *){} }};",
            target_name, len_name, shp_name, data_name
        ));
    }

    fn visit_call(&mut self, call: &Call<'ast, TypedAst>) {
        if let CallTarget::TraitMethod { trait_name, method_name } = &call.id {
            let args: Vec<String> = call.args.iter()
                .map(|arg| self.render_expr(&Expr::Id(*arg)))
                .collect();
            let arg_types: Vec<Type> = call.args.iter().map(|id| self.id_type(id)).collect();
            let shim_name = self.trait_shim_name(trait_name, method_name, &arg_types);
            let rendered = format!("IMP_{}({})", shim_name, args.join(", "));
            self.expr_stack.push(rendered);
            return;
        }

        let base_name = TypedAst::dispatch_name(&call.id);
        let arg_types: Vec<Type> = call.args.iter().map(|id| match id {
            Id::Arg(i) => self.arg_types[*i].clone(),
            Id::Var(v) => v.ty.clone(),
            Id::Dim(_) => Type::scalar(BaseType::Usize),
            Id::DimAt(_, _) => Type::scalar(BaseType::Usize),
            Id::Shp(_) => Type {
                ty: BaseType::Usize,
                shape: ShapePattern::Axes(vec![AxisPattern::Dim(DimPattern::Any)]),
                knowledge: TypeKnowledge::AKD,
            },
        }).collect();
        let name = rename_fundefs::mangle_call_name(&base_name, &arg_types);
        let args: Vec<String> = call.args.iter()
            .map(|arg| self.render_expr(&Expr::Id(*arg)))
            .collect();
        self.expr_stack.push(format!("IMP_{}({})", name, args.join(", ")));
    }

    fn visit_prf_call(&mut self, prf_call: &PrfCall<'ast, TypedAst>) {
        use Prf::*;

        let args: Vec<String> = prf_call.args.iter()
            .map(|arg| self.render_expr(&Expr::Id(*arg)))
            .collect();

        let rendered = match prf_call.id {
            AddSxS => format!("{} + {}", args[0], args[1]),
            SubSxS => format!("{} - {}", args[0], args[1]),
            MulSxS => format!("{} * {}", args[0], args[1]),
            DivSxS => format!("{} / {}", args[0], args[1]),
            LtSxS => format!("{} < {}", args[0], args[1]),
            LeSxS => format!("{} <= {}", args[0], args[1]),
            GtSxS => format!("{} > {}", args[0], args[1]),
            GeSxS => format!("{} >= {}", args[0], args[1]),
            EqSxS => format!("{} == {}", args[0], args[1]),
            NeSxS => format!("{} != {}", args[0], args[1]),
            NegS => format!("-{}", args[0]),
            NotS => format!("!{}", args[0]),
            SelAxV => {
                let arr_id = prf_call.args[0];
                let idx_id = prf_call.args[1];
                if self.id_is_any(&arr_id) || self.id_is_any(&idx_id) {
                    panic!("dynamic union values are not yet supported in primitive selection during C codegen");
                }
                let arr = self.nameof(&arr_id);
                let idx = self.nameof(&idx_id);
                let elem_base = elem_ctype_of_id(&arr_id);
                let flat_fn = flat_index_fn_of_id(&idx_id);
                format!("(({elem_base} *){arr}.data)[{flat_fn}({arr}, {idx})]")
            }
        };

        self.expr_stack.push(rendered);
    }

    fn visit_id(&mut self, id: &Id<'ast, Self::Ast>) {
        match id {
            Id::Arg(i) => self.expr_stack.push(self.arg_names[*i].clone()),
            Id::Var(lvis) => self.expr_stack.push(lvis.name.clone()),
            Id::Dim(i) => {
                let name = format!("{}.dim", self.arg_names[*i]);
                self.expr_stack.push(name);
            }
            Id::DimAt(i, k) => {
                let name = format!("{}.shp[{k}]", self.arg_names[*i]);
                self.expr_stack.push(name);
            }
            Id::Shp(i) => {
                let arg = self.arg_names[*i].clone();
                self.shp_uid += 1;
                let uid = self.shp_uid;
                let meta = format!("_shp{uid}_meta");
                let data = format!("_shp{uid}_data");
                let wrap = format!("_shp{uid}");
                self.push_line(&format!("size_t *{meta} = (size_t *)malloc(sizeof(size_t));"));
                self.push_line(&format!("*{meta} = {arg}.dim;"));
                self.push_line(&format!("size_t *{data} = (size_t *)malloc({arg}.dim * sizeof(size_t));"));
                self.push_line(&format!("for (size_t _i = 0; _i < {arg}.dim; _i += 1) {{ {data}[_i] = {arg}.shp[_i]; }}"));
                self.push_line(&format!(
                    "ImpArrayRaw {wrap} = (ImpArrayRaw) {{ .len = {arg}.dim, .dim = 1, .shp = {meta}, .data = (void *){data} }};",
                ));
                self.expr_stack.push(wrap);
            }
        }
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
        Id::Dim(_) | Id::Shp(_) | Id::DimAt(_, _) => "size_t",
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
        Id::Dim(_) | Id::Shp(_) | Id::DimAt(_, _) => BaseType::Usize,
    }
}

fn type_of_id_in_context(id: &Id<'_, TypedAst>, args: &[&Farg]) -> Type {
    match id {
        Id::Arg(i) => args[*i].ty.clone(),
        Id::Var(v) => v.ty.clone(),
        Id::Dim(_) | Id::DimAt(_, _) => Type::scalar(BaseType::Usize),
        Id::Shp(_) => Type {
            ty: BaseType::Usize,
            shape: ShapePattern::Axes(vec![AxisPattern::Dim(DimPattern::Any)]),
            knowledge: TypeKnowledge::AKD,
        },
    }
}

