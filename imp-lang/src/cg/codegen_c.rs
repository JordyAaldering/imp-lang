use crate::{ast::*, cg::rename_fundefs, Visit};

pub fn emit_c(ast: &mut Program<'static, TypedAst>, module_name: String) -> String {
    let mut cg = CompileC::new(module_name);
    cg.visit_program(ast);
    cg.finish()
}

pub struct CompileC {
    output: String,
    module_name: String,
    arg_names: Vec<String>,
    arg_types: Vec<Type>,
    ret_type: Option<Type>,
    expr_stack: Vec<String>,
    lhs_target: Option<(String, Type)>,
    indent: usize,
    shp_uid: usize,
    tensor_uid: usize,
}

impl CompileC {
    pub fn new(module_name: String) -> Self {
        Self {
            output: String::new(),
            module_name,
            arg_names: Vec::new(),
            arg_types: Vec::new(),
            ret_type: None,
            expr_stack: Vec::new(),
            lhs_target: None,
            indent: 0,
            shp_uid: 0,
            tensor_uid: 0,
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

    fn id_type(&self, id: &Id<'_, TypedAst>) -> Type {
        match id {
            Id::Arg(i) => self.arg_types[*i].clone(),
            Id::Var(v) => v.ty.clone(),
        }
    }

    fn nameof(&mut self, id: &Id<'_, TypedAst>) -> String {
        match id {
            Id::Arg(i) => self.arg_names[*i].clone(),
            Id::Var(v) => v.name.clone(),
        }
    }

    fn emit_function_prototype(&mut self, fundef: &Fundef<'_, TypedAst>) {
        let args: Vec<String> = fundef
            .args
            .iter()
            .map(|arg| format!("{} {}", full_ctype(&arg.ty), arg.id))
            .collect();
        self.output.push_str(&format!(
            "{} IMP_{}({});\n",
            full_ctype(&fundef.ret_type),
            fundef.name,
            args.join(", ")
        ));
    }

    fn emit_wrapper_prototype(&mut self, base_name: &str, sig: &BaseSignature, ret_ty: &BaseType) {
        let sig_str = sig.base_types.iter().map(base_rstype).collect::<Vec<_>>();
        let fargs: Vec<String> = sig.base_types
            .iter()
            .enumerate()
            .map(|(i, base)| format!("{} arg{i}", dyn_ctype(base)))
            .collect();
        self.push_line(&format!("{} IMP_{}_{}({});",
            dyn_ctype(ret_ty), base_name, sig_str.join("_"), fargs.join(", ")));
    }

    fn emit_wrapper_function(&mut self, base_name: &str, sig: &BaseSignature, family: &Vec<Fundef<'_, TypedAst>>) {
        let sig_str = sig.base_types.iter().map(base_rstype).collect::<Vec<_>>();
        let fargs: Vec<String> = sig.base_types
            .iter()
            .enumerate()
            .map(|(i, base)| format!("{} arg{i}", dyn_ctype(base)))
            .collect();

        self.push_line(&format!("{} IMP_{}_{}({}) {{",
            dyn_ctype(&family[0].ret_type.ty), base_name, sig_str.join("_"), fargs.join(", ")));

        self.indent += 1;
        for (idx, fundef) in family.iter().enumerate() {
            let condition = fundef.args
                .iter()
                .enumerate()
                .map(|(i, arg)| shape_match_condition(&arg.ty.shape, &format!("arg{i}")))
                .collect::<Vec<_>>()
                .join(" && ");

            if idx > 0 {
                self.push_line("else ");
            }
            self.push_line(&format!("if ({condition}) {{"));
            self.indent += 1;

            let call_args: Vec<String> = fundef.args
                .iter()
                .enumerate()
                .map(|(i, arg)| wrapper_call_arg(&arg.ty.shape, &format!("arg{i}")))
                .collect();
            let call_expr = format!("IMP_{}({})", fundef.name, call_args.join(", "));

            if matches!(fundef.ret_type.shape, TypePattern::Any) {
                self.push_line(&format!("return {call_expr};"));
            } else if fundef.ret_type.is_array() {
                let dyn_ty = dyn_ctype(&fundef.ret_type.ty);
                self.push_line(&format!("ImpArrayRaw __ret = {call_expr};"));
                self.push_line(&format!(
                    "return ({dyn_ty}) {{ .is_array = true, .data.array = __ret }};"
                ));
            } else {
                let dyn_ty = dyn_ctype(&fundef.ret_type.ty);
                self.push_line(&format!("{} __ret = {call_expr};", base_ctype(&fundef.ret_type)));
                self.push_line(&format!(
                    "return ({dyn_ty}) {{ .is_array = false, .data.scalar = __ret }};"
                ));
            }

            self.indent -= 1;
            self.push_line("}");
        }

        self.push_line(&format!("fprintf(stderr, \"runtime overload dispatch failed: {}\\n\");", base_name));
        self.push_line("abort();");

        self.indent -= 1;
        self.push_line("}");
    }

    fn emit_return(&mut self, ret: Id<'_, TypedAst>) {
        let name = self.render_expr(&Expr::Id(ret));
        let declared_ty = self.ret_type.clone().unwrap_or_else(|| self.id_type(&ret));
        let value_ty = self.id_type(&ret);

        if matches!(declared_ty.shape, TypePattern::Any) {
            let dyn_ty = dyn_ctype(&declared_ty.ty);
            self.push_line(&format!("if ({name}.is_array) {{"));
            self.indent += 1;
            self.push_line(&format!("{dyn_ty} out = {name};"));
            self.push_line(&format!(
                "out.data.array = imp_clone_array_raw({name}.data.array, sizeof({}));",
                base_ctype(&declared_ty)
            ));
            self.push_line("return out;");
            self.indent -= 1;
            self.push_line("}");
            self.push_line(&format!("return {};", name));
        } else if declared_ty.is_array() {
            if matches!(value_ty.shape, TypePattern::Any) {
                self.push_line(&format!("if (!{name}.is_array) {{"));
                self.indent += 1;
                self.push_line("fprintf(stderr, \"return type mismatch: expected array\\n\");");
                self.push_line("abort();");
                self.indent -= 1;
                self.push_line("}");
                self.push_line(&format!(
                    "return imp_clone_array_raw({name}.data.array, sizeof({}));",
                    base_ctype(&declared_ty)
                ));
            } else {
                self.push_line(&format!(
                    "return imp_clone_array_raw({}, sizeof({}));",
                    name,
                    base_ctype(&declared_ty)
                ));
            }
        } else {
            if matches!(value_ty.shape, TypePattern::Any) {
                self.push_line(&format!("if ({name}.is_array) {{"));
                self.indent += 1;
                self.push_line("fprintf(stderr, \"return type mismatch: expected scalar\\n\");");
                self.push_line("abort();");
                self.indent -= 1;
                self.push_line("}");
                self.push_line(&format!("return {name}.data.scalar;"));
            } else {
                self.push_line(&format!("return {};", name));
            }
        }
    }
}

const HEADER: &str = r#"
#include <stdio.h>
#include <string.h>

static size_t imp_flat_index(ImpArrayRaw arr, ImpArrayRaw idx) {
    size_t flat = 0;
    for (size_t d = 0; d < idx.len; d += 1) {
        flat = flat * arr.shp[d] + ((size_t *)idx.data)[d];
    }
    return flat;
}

static ImpArrayRaw imp_clone_array_raw(ImpArrayRaw src, size_t elem_size) {
    size_t *shp = src.dim == 0 ? NULL : (size_t *)malloc(src.dim * sizeof(size_t));
    if (src.dim > 0) { memcpy(shp, src.shp, src.dim * sizeof(size_t)); }
    void *data = src.len == 0 ? NULL : malloc(src.len * elem_size);
    if (src.len > 0) { memcpy(data, src.data, src.len * elem_size); }
    return (ImpArrayRaw) { .len = src.len, .dim = src.dim, .shp = shp, .data = data };
}
"#;

impl<'ast> Visit<'ast> for CompileC {
    type Ast = TypedAst;

    fn visit_program(&mut self, program: &Program<'ast, TypedAst>) {
        self.output.push_str(&format!("#include \"{}.h\"\n", self.module_name));
        self.output.push_str(HEADER);

        for (_name, overloads) in &program.overloads {
            for (_sig, fundefs) in overloads {
                for fundef in fundefs {
                    self.output.push('\n');
                    self.emit_function_prototype(fundef);
                }
            }
        }

        for (name, overloads) in &program.overloads {
            let name = name.strip_prefix('@').unwrap_or(name).to_owned();
            for (sig, fundefs) in overloads {
                if overloads.len() > 1 || fundefs.len() > 1 {
                    self.output.push('\n');
                    self.emit_wrapper_prototype(&name, sig, &fundefs[0].ret_type.ty);
                }
            }
        }


        for (_name, overloads) in &program.overloads {
            for (_sig, fundefs) in overloads {
                for fundef in fundefs {
                    self.output.push('\n');
                    self.visit_fundef(fundef);
                }
            }
        }

        for (name, overloads) in &program.overloads {
            let name = name.strip_prefix('@').unwrap_or(name).to_owned();
            for (sig, fundefs) in overloads {
                if overloads.len() > 1 || fundefs.len() > 1 {
                    self.output.push('\n');
                    self.emit_wrapper_function(&name, sig, fundefs);
                }
            }
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, TypedAst>) {
        self.arg_names = fundef.args.iter().map(|arg| arg.id.clone()).collect();
        self.arg_types = fundef.args.iter().map(|arg| arg.ty.clone()).collect();
        self.ret_type = Some(fundef.ret_type.clone());
        let args: Vec<String> = fundef.args.iter()
            .map(|arg| format!("{} {}", full_ctype(&arg.ty), arg.id))
            .collect();

        self.push_line(&format!(
            "{} IMP_{}({}) {{",
            full_ctype(&fundef.ret_type), fundef.name, args.join(", ")
        ));

        self.indent += 1;
        for assign in &fundef.shape_prelude {
            self.visit_assign(assign);
        }
        for stmt in &fundef.body.stmts {
            self.visit_stmt(stmt);
        }
        self.emit_return(fundef.body.ret);
        self.indent -= 1;

        self.push_line("}");
        self.ret_type = None;
    }

    fn visit_body(&mut self, _body: &Body<'ast, Self::Ast>) {
        unreachable!("needs to be implemented in a case-by-case basis")
    }

    fn visit_assign(&mut self, assign: &Assign<'ast, Self::Ast>) {
        if let Expr::Tensor(tensor) = assign.expr {
            self.lhs_target = Some((assign.lhs.name.clone(), assign.lhs.ty.clone()));
            self.visit_tensor(tensor);
            self.lhs_target = None;
            return;
        }

        if let Expr::Fold(fold) = assign.expr {
            self.lhs_target = Some((assign.lhs.name.clone(), assign.lhs.ty.clone()));
            self.visit_fold(fold);
            self.lhs_target = None;
            return;
        }

        if let Expr::Array(array) = assign.expr {
            self.lhs_target = Some((assign.lhs.name.clone(), assign.lhs.ty.clone()));
            self.visit_array(array);
            self.lhs_target = None;
            return;
        }

        let rhs = self.render_expr(assign.expr);
        self.push_line(&format!("{} {} = {};", full_ctype(&assign.lhs.ty), assign.lhs.name, rhs));
    }

    fn visit_printf(&mut self, printf: &Printf<'ast, Self::Ast>) {
        let id = self.nameof(&printf.id);
        self.push_line(&format!("printf(\"Hello, {}\\n\");", id));
    }

    fn visit_cond(&mut self, cond: &Cond<'ast, Self::Ast>) {
        if cond.then_branch.stmts.is_empty() && cond.else_branch.stmts.is_empty() {
            let c = self.nameof(&cond.cond);
            let t = self.nameof(&cond.then_branch.ret);
            let f = self.nameof(&cond.else_branch.ret);
            self.expr_stack.push(format!("{} ? {} : {}", c, t, f));
        } else {
            self.push_line(&format!("{} cond_ret;", full_ctype(&self.id_type(&cond.then_branch.ret))));

            let c = self.nameof(&cond.cond);
            self.push_line(&format!("if ({}) {{", c));
            self.indent += 1;

            for stmt in &cond.then_branch.stmts {
                self.visit_stmt(stmt);
            }
            let t = self.nameof(&cond.then_branch.ret);
            self.push_line(&format!("cond_ret = {};", t));

            self.indent -= 1;
            self.push_line("} else {");
            self.indent += 1;

            for stmt in &cond.else_branch.stmts {
                self.visit_stmt(stmt);
            }
            let f = self.nameof(&cond.else_branch.ret);
            self.push_line(&format!("cond_ret = {};", f));

            self.indent -= 1;
            self.push_line("}");

            self.expr_stack.push("cond_ret".to_string());
        }
    }

    fn visit_fold(&mut self, fold: &Fold<'ast, Self::Ast>) {
        let (target_name, target_ty, push_result) = if let Some((name, ty)) = self.lhs_target.clone() {
            (name, ty, false)
        } else {
            self.tensor_uid += 1;
            (format!("_fold_{}", self.tensor_uid), self.id_type(&fold.neutral), true)
        };

        let iv_name = fold.selection.iv.name.clone();
        let rank = fold.selection.iv.ty.rank()
            .expect("fold selection iv must have a statically-known rank for C codegen") as usize;

        self.tensor_uid += 1;
        let t_uid = self.tensor_uid;

        let neutral_expr = self.render_expr(&Expr::Id(fold.neutral));
        self.push_line(&format!("{} {} = {};", full_ctype(&target_ty), target_name, neutral_expr));

        for d in 0..rank {
            if let Some(lb) = &fold.selection.lb {
                let lb_name = self.nameof(lb);
                self.push_line(&format!("size_t {iv_name}_lb{d}_{t_uid} = ((size_t *){lb_name}.data)[{d}];"));
            }
            let ub_name = self.nameof(&fold.selection.ub);
            self.push_line(&format!("size_t {iv_name}_ub{d}_{t_uid} = ((size_t *){ub_name}.data)[{d}];"));
        }

        for d in 0..rank {
            if fold.selection.lb.is_some() {
                self.push_line(&format!("for (size_t {iv_name}_{d}_{t_uid} = {iv_name}_lb{d}_{t_uid}; {iv_name}_{d}_{t_uid} < {iv_name}_ub{d}_{t_uid}; {iv_name}_{d}_{t_uid} += 1) {{"));
            } else {
                self.push_line(&format!("for (size_t {iv_name}_{d}_{t_uid} = 0; {iv_name}_{d}_{t_uid} < {iv_name}_ub{d}_{t_uid}; {iv_name}_{d}_{t_uid} += 1) {{"));
            }
            self.indent += 1;
        }

        let iv_elem = base_ctype(&fold.selection.iv.ty);
        let iv_components: Vec<String> = (0..rank)
            .map(|d| format!("({iv_elem}){iv_name}_{d}_{t_uid}"))
            .collect();
        self.push_line(&format!(
            "{iv_elem} {iv_name}_data_{t_uid}[{rank}] = {{ {} }};",
            iv_components.join(", ")
        ));
        self.push_line(&format!("size_t {iv_name}_shp_arr_{t_uid}[1] = {{ {rank} }};"));
        self.push_line(&format!(
            "ImpArrayRaw {iv_name} = (ImpArrayRaw) {{ .len = {rank}, .shp = {iv_name}_shp_arr_{t_uid}, .dim = 1, .data = (void *){iv_name}_data_{t_uid} }};"
        ));

        for stmt in &fold.selection.body.stmts {
            self.visit_stmt(stmt);
        }

        let sel_expr = self.render_expr(&Expr::Id(fold.selection.body.ret));

        let (fold_name, call_args) = match &fold.foldfun {
            FoldFun::Name(id) => {
                let name = match id {
                    CallTarget::Function(f) => rename_fundefs::mangle_fundef_name(&f.name, &f.args),
                };
                (name, vec![target_name.clone(), sel_expr])
            }
            FoldFun::Apply { id, args } => {
                let name = match id {
                    CallTarget::Function(f) => rename_fundefs::mangle_fundef_name(&f.name, &f.args),
                };
                let mut hole = 0usize;
                let mut out = Vec::with_capacity(args.len());
                for arg in args {
                    match arg {
                        FoldFunArg::Placeholder => {
                            hole += 1;
                            if hole == 1 {
                                out.push(target_name.clone());
                            } else {
                                out.push(sel_expr.clone());
                            }
                        }
                        FoldFunArg::Bound(bound) => {
                            out.push(self.render_expr(&Expr::Id(bound.clone())));
                        }
                    }
                }
                (name, out)
            }
        };

        self.push_line(&format!("{} = IMP_{}({});", target_name, fold_name, call_args.join(", ")));

        for _ in 0..rank {
            self.indent -= 1;
            self.push_line("}");
        }

        if push_result {
            self.expr_stack.push(target_name);
        }
    }

    fn visit_tensor(&mut self, tensor: &Tensor<'ast, Self::Ast>) {
        let (target_name, target_ty) = self.lhs_target.clone().expect("tensor target must be set");
        let base = base_ctype(&target_ty);
        let iv_name = tensor.iv.name.clone();

        let rank = tensor.iv.ty.rank()
            .expect("tensor iv must have a statically-known rank for C codegen") as usize;

        self.tensor_uid += 1;
        let t_uid = self.tensor_uid;

        // Extract scalar lower/upper bound per dimension.
        for d in 0..rank {
            if let Some(lb) = &tensor.lb {
                let lb_name = self.nameof(lb);
                self.push_line(&format!("size_t {iv_name}_lb{d}_{t_uid} = ((size_t *){lb_name}.data)[{d}];"));
            }
            let ub_name = self.nameof(&tensor.ub);
            self.push_line(&format!("size_t {iv_name}_ub{d}_{t_uid} = ((size_t *){ub_name}.data)[{d}];"));
        }

        // Total element count in the result (product of extents).
        let len_name  = format!("{target_name}_len");
        let data_name = format!("{target_name}_data");
        let shp_name  = format!("{target_name}_shp");
        let extents: Vec<String> = (0..rank)
            .map(|d| {
                if tensor.lb.is_some() {
                    format!("({iv_name}_ub{d}_{t_uid} - {iv_name}_lb{d}_{t_uid})")
                } else {
                    format!("{iv_name}_ub{d}_{t_uid}")
                }
            })
            .collect();
        let total_len = if extents.is_empty() { "1".to_owned() } else { extents.join(" * ") };
        self.push_line(&format!("size_t {len_name} = {total_len};"));
        self.push_line(&format!("{base} *{data_name} = ({base} *)malloc({len_name} * sizeof({base}));"));

        // Heap-allocate the result shape array.
        self.push_line(&format!("size_t *{shp_name} = (size_t *)malloc({rank} * sizeof(size_t));"));
        for d in 0..rank {
            if tensor.lb.is_some() {
                self.push_line(&format!("{shp_name}[{d}] = {iv_name}_ub{d}_{t_uid} - {iv_name}_lb{d}_{t_uid};"));
            } else {
                self.push_line(&format!("{shp_name}[{d}] = {iv_name}_ub{d}_{t_uid};"));
            }
        }

        // Generate k nested for-loops.
        for d in 0..rank {
            if tensor.lb.is_some() {
                self.push_line(&format!("for (size_t {iv_name}_{d}_{t_uid} = {iv_name}_lb{d}_{t_uid}; {iv_name}_{d}_{t_uid} < {iv_name}_ub{d}_{t_uid}; {iv_name}_{d}_{t_uid} += 1) {{"));
            } else {
                self.push_line(&format!("for (size_t {iv_name}_{d}_{t_uid} = 0; {iv_name}_{d}_{t_uid} < {iv_name}_ub{d}_{t_uid}; {iv_name}_{d}_{t_uid} += 1) {{"));
            }
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
            "ImpArrayRaw {iv_name} = (ImpArrayRaw) {{ .len = {rank}, .shp = {iv_name}_shp_arr_{t_uid}, .dim = 1, .data = (void *){iv_name}_data_{t_uid} }};"
        ));

        // Row-major flat index: Σ (iv_d - lb_d) * stride_d
        let flat_terms: Vec<String> = (0..rank).map(|d| {
            let stride: Vec<String> = (d + 1..rank)
                .map(|j| {
                    if tensor.lb.is_some() {
                        format!("({iv_name}_ub{j}_{t_uid} - {iv_name}_lb{j}_{t_uid})")
                    } else {
                        format!("{iv_name}_ub{j}_{t_uid}")
                    }
                })
                .collect();

            let stride_expr = if stride.is_empty() { "1".to_owned() } else { stride.join(" * ") };

            if tensor.lb.is_some() {
                format!("({iv_name}_{d}_{t_uid} - {iv_name}_lb{d}_{t_uid}) * {stride_expr}")
            } else {
                format!("{iv_name}_{d}_{t_uid} * {stride_expr}")
            }
        }).collect();
        let flat_expr = if flat_terms.is_empty() { "0".to_owned() } else { flat_terms.join(" + ") };
        self.push_line(&format!("size_t {iv_name}_flat = {flat_expr};"));

        // Body statements.
        for stmt in &tensor.body.stmts {
            self.visit_stmt(stmt);
        }

        // Store element into the flat result buffer.
        let mut ret = self.render_expr(&Expr::Id(tensor.body.ret));
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

        self.push_line(&format!("size_t {} = {};", len_name, array.elems.len()));
        self.push_line(&format!("{} *{} = ({} *)malloc({} * sizeof({}));", base, data_name, base, len_name, base));

        for (i, value) in array.elems.iter().enumerate() {
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
        let (target_base_name, target_symbol) = match &call.id {
            CallTarget::Function(f) => (
                f.name.clone(),
                rename_fundefs::mangle_fundef_name(&f.name, &f.args),
            ),
        };
        let arg_types: Vec<Type> = call.args.iter().map(|id| match id {
            Id::Arg(i) => self.arg_types[*i].clone(),
            Id::Var(v) => v.ty.clone(),
        }).collect();

        let needs_runtime_wrapper = arg_types.iter().any(|t| matches!(t.shape, TypePattern::Any));
        let name = if needs_runtime_wrapper {
            let root = target_base_name.split("__").next().unwrap_or(&target_base_name);
            let any_types: Vec<Type> = arg_types
                .iter()
                .map(|t| Type { ty: t.ty.clone(), shape: TypePattern::Any })
                .collect();
            rename_fundefs::mangle_call_name(root, &any_types)
        } else {
            target_symbol
        };

        let args: Vec<String> = call.args.iter()
            .map(|arg| self.render_expr(&Expr::Id(*arg)))
            .collect();
        self.expr_stack.push(format!("IMP_{}({})", name, args.join(", ")));
    }

    fn visit_prf_call(&mut self, prf_call: &PrfCall<'ast, TypedAst>) {
        use PrfCall::*;
        let rendered = match prf_call {
            DimA(arr) => {
                let arg = self.render_expr(&Expr::Id(*arr));
                format!("{arg}.dim")
            }
            ShapeA(arr) => {
                let arg = self.render_expr(&Expr::Id(*arr));
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
                wrap
            }
            SelVxA(idx, arr) => {
                let arr_name = self.render_expr(&Expr::Id(*arr));
                let idx_name = self.render_expr(&Expr::Id(*idx));
                let elem_base = elem_ctype_of_id(arr);
                format!("(({elem_base} *){arr_name}.data)[imp_flat_index({arr_name}, {idx_name})]")
            }
            AddSxS(a, b) => format!("{} + {}", self.render_expr(&Expr::Id(*a)), self.render_expr(&Expr::Id(*b))),
            SubSxS(a, b) => format!("{} - {}", self.render_expr(&Expr::Id(*a)), self.render_expr(&Expr::Id(*b))),
            MulSxS(a, b) => format!("{} * {}", self.render_expr(&Expr::Id(*a)), self.render_expr(&Expr::Id(*b))),
            DivSxS(a, b) => format!("{} / {}", self.render_expr(&Expr::Id(*a)), self.render_expr(&Expr::Id(*b))),
            LtSxS(a, b) => format!("{} < {}", self.render_expr(&Expr::Id(*a)), self.render_expr(&Expr::Id(*b))),
            LeSxS(a, b) => format!("{} <= {}", self.render_expr(&Expr::Id(*a)), self.render_expr(&Expr::Id(*b))),
            GtSxS(a, b) => format!("{} > {}", self.render_expr(&Expr::Id(*a)), self.render_expr(&Expr::Id(*b))),
            GeSxS(a, b) => format!("{} >= {}", self.render_expr(&Expr::Id(*a)), self.render_expr(&Expr::Id(*b))),
            EqSxS(a, b) => format!("{} == {}", self.render_expr(&Expr::Id(*a)), self.render_expr(&Expr::Id(*b))),
            NeSxS(a, b) => format!("{} != {}", self.render_expr(&Expr::Id(*a)), self.render_expr(&Expr::Id(*b))),
            NegS(a) => format!("-{}", self.render_expr(&Expr::Id(*a))),
            NotS(a) => format!("!{}", self.render_expr(&Expr::Id(*a))),
        };
        self.expr_stack.push(rendered);
    }

    fn visit_id(&mut self, id: &Id<'ast, Self::Ast>) {
        match id {
            Id::Arg(i) => self.expr_stack.push(self.arg_names[*i].clone()),
            Id::Var(lvis) => self.expr_stack.push(lvis.name.clone()),
        }
    }

    fn visit_const(&mut self, c: &Const) {
        use Const::*;
        let s = match c {
            Bool(v) => v.to_string(),
            I32(v) => v.to_string(),
            I64(v) => v.to_string(),
            U32(v) => v.to_string(),
            U64(v) => v.to_string(),
            Usize(v) => v.to_string(),
            F32(v) => v.to_string(),
            F64(v) => v.to_string(),
        };
        self.expr_stack.push(s)
    }
}

fn base_rstype(ty: &BaseType) -> String {
    use BaseType::*;
    match ty {
        Bool => "bool".to_owned(),
        I32 => "i32".to_owned(),
        I64 => "i64".to_owned(),
        U32 => "u32".to_owned(),
        U64 => "u64".to_owned(),
        Usize => "usize".to_owned(),
        F32 => "f32".to_owned(),
        F64 => "f64".to_owned(),
        Udf(udf) => udf.to_owned(),
    }
}

fn base_ctype(ty: &Type) -> String {
    use BaseType::*;
    match &ty.ty {
        I32 => "int32_t".to_owned(),
        I64 => "int64_t".to_owned(),
        U32 => "uint32_t".to_owned(),
        U64 => "uint64_t".to_owned(),
        Usize => "size_t".to_owned(),
        F32 => "float".to_owned(),
        F64 => "double".to_owned(),
        Bool => "bool".to_owned(),
        Udf(udf) => udf.to_owned(),
    }
}

fn full_ctype(ty: &Type) -> String {
    if matches!(ty.shape, TypePattern::Any) {
        use BaseType::*;
        return match &ty.ty {
            I32 => "ImpDynI32".to_owned(),
            I64 => "ImpDynI64".to_owned(),
            U32 => "ImpDynU32".to_owned(),
            U64 => "ImpDynU64".to_owned(),
            Usize => "ImpDynUsize".to_owned(),
            F32 => "ImpDynF32".to_owned(),
            F64 => "ImpDynF64".to_owned(),
            Bool => "ImpDynBool".to_owned(),
            Udf(udf) => format!("ImpDyn{}", udf),
        }
    } else if ty.is_array() {
        "ImpArrayRaw".to_owned()
    } else {
        base_ctype(ty).to_owned()
    }
}

fn dyn_ctype(base: &BaseType) -> String {
    full_ctype(&Type {
        ty: base.clone(),
        shape: TypePattern::Any,
    })
}

fn shape_match_condition(shape: &TypePattern, arg: &str) -> String {
    match shape {
        TypePattern::Scalar => format!("!{arg}.is_array"),
        TypePattern::Any => "1".to_owned(),
        TypePattern::Axes(axes) => {
            if axes.iter().any(|ax| matches!(ax, AxisPattern::Rank(_))) {
                return format!("{arg}.is_array");
            }

            let mut checks = vec![
                format!("{arg}.is_array"),
                format!("{arg}.data.array.dim == {}", axes.len()),
            ];
            for (i, axis) in axes.iter().enumerate() {
                if let AxisPattern::Dim(DimPattern::Known(v)) = axis {
                    checks.push(format!("{arg}.data.array.shp[{i}] == {v}"));
                }
            }
            checks.join(" && ")
        }
    }
}

fn wrapper_call_arg(shape: &TypePattern, arg: &str) -> String {
    match shape {
        TypePattern::Scalar => format!("{arg}.data.scalar"),
        TypePattern::Any => arg.to_owned(),
        TypePattern::Axes(_) => format!("{arg}.data.array"),
    }
}

fn elem_ctype_of_id(id: &Id<'_, TypedAst>) -> String {
    match id {
        Id::Arg(_) => "uint32_t".to_owned(),  // args used directly as array bounds are uncommon
        Id::Var(v) => base_ctype(&v.ty),
    }
}
