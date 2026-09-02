use crate::ast::*;

const HEADER: &str =
r#"#include <stdio.h>
#include <string.h>

// Allocate memory for an array's shape vector, for the given dimension.
#define IMP_ALLOC_SHAPE_VEC(DIM) ((size_t *)malloc((DIM) * sizeof(size_t)))

// Allocate memory to hold the array's data, for the given element type and number of elements.
#define IMP_ALLOC_DATA(TYPE, LEN) ((TYPE *)malloc((LEN) * sizeof(TYPE)))

#define IMP_MK_ARRAY(LEN, DIM, SHP, DATA) ((ImpArrayRaw){ .len=(LEN), .dim=(DIM), .shp=(SHP), .data=(void *)(DATA) })

static size_t imp_flat_index(ImpArrayRaw arr, ImpArrayRaw idx) {
    size_t flat = 0;
    for (size_t d = 0; d < idx.len; d++) {
        flat = flat * arr.shp[d] + ((size_t *)idx.data)[d];
    }
    return flat;
}

static ImpArrayRaw imp_clone_array_raw(ImpArrayRaw src, size_t elem_size) {
    size_t *shp = src.dim == 0 ? NULL : IMP_ALLOC_SHAPE_VEC(src.dim);
    if (src.dim > 0) { memcpy(shp, src.shp, src.dim * sizeof(size_t)); }
    void *data = src.len == 0 ? NULL : malloc(src.len * elem_size);
    if (src.len > 0) { memcpy(data, src.data, src.len * elem_size); }
    return IMP_MK_ARRAY(src.len, src.dim, shp, data);
}
"#;

pub fn emit_c(ast: &mut Program<'_, TypedAst>, module_name: String) -> String {
    let mut cg = CompileC::default();
    cg.module_name = module_name;
    cg.trav_program(ast);
    cg.output
}

#[derive(Default)]
struct CompileC {
    output: String,
    module_name: String,
    fundef_names: Vec<String>,
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
    fn push_line(&mut self, line: &str) {
        self.output.push_str(&"    ".repeat(self.indent));
        self.output.push_str(line);
        self.output.push('\n');
    }

    fn render_id(&mut self, mut id: Id<'_, TypedAst>) -> String {
        self.trav_id(&mut id);
        self.expr_stack.pop().expect("ID stack underflow")
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
            .map(|arg| format!("{} {}", arg.ty.ctype(), arg.id))
            .collect();
        self.output.push_str(&format!(
            "{} IMP_{}({});\n",
            fundef.ret_type.ctype(),
            fundef.name,
            args.join(", ")
        ));
    }

    fn emit_wrapper_prototype(&mut self, base_name: &str, sig: &BaseSignature, _ret_ty: &BaseType) {
        let sig_str = sig.base_types.iter().map(BaseType::ctype).collect::<Vec<_>>();
        let fargs: Vec<String> = sig.base_types
            .iter()
            .enumerate()
            .map(|(i, _base)| format!("ImpArrayRaw arg{i}"))
            .collect();
        self.push_line(&format!("ImpArrayRaw IMP_{}_{}({});",
            base_name, sig_str.join("_"), fargs.join(", ")));
    }

    fn emit_wrapper_function(&mut self, base_name: &str, sig: &BaseSignature, family: &Vec<&Fundef<'_, TypedAst>>) {
        let sig_str = sig.base_types.iter().map(BaseType::ctype).collect::<Vec<_>>();
        let fargs: Vec<String> = sig.base_types
            .iter()
            .enumerate()
            .map(|(i, _base)| format!("ImpArrayRaw arg{i}"))
            .collect();

        let first = family[0];
        self.push_line(&format!("ImpArrayRaw IMP_{}_{}({}) {{",
            base_name, sig_str.join("_"), fargs.join(", ")));

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
                .map(|(i, arg)| wrapper_call_arg(&arg.ty.shape, &format!("arg{i}"), &arg.ty.basetype))
                .collect();
            let call_expr = format!("IMP_{}({})", fundef.name, call_args.join(", "));

            if fundef.ret_type.is_array() {
                self.push_line(&format!("return {call_expr};"));
            } else {
                // Scalar return: wrap in a 0-d ImpArrayRaw (dim=0, len=1, shp=NULL)
                let base = fundef.ret_type.basetype.ctype();
                self.push_line(&format!("{base} __ret_val = {call_expr};"));
                self.push_line(&format!("{base} *__ret_data = IMP_ALLOC_DATA({base}, 1);"));
                self.push_line("*__ret_data = __ret_val;");
                self.push_line("return IMP_MK_ARRAY(1, 0, NULL, __ret_data);");
            }

            self.indent -= 1;
            self.push_line("}");
        }

        // suppress unused-variable warning for the last fundef we borrowed
        let _ = first;
        self.push_line(&format!("fprintf(stderr, \"runtime overload dispatch failed: {}\\n\");", base_name));
        self.push_line("abort();");

        self.indent -= 1;
        self.push_line("}");
    }

    fn emit_return(&mut self, ret: Id<'_, TypedAst>) {
        let name = self.render_id(ret);
        let declared_ty = self.ret_type.clone().unwrap_or_else(|| self.id_type(&ret));

        if declared_ty.is_array() {
            self.push_line(&format!(
                "return imp_clone_array_raw({}, sizeof({}));",
                name,
                declared_ty.basetype.ctype()
            ));
        } else {
            self.push_line(&format!("return {};", name));
        }
    }
}

impl<'ast> Traverse<'ast> for CompileC {
    type Ast = TypedAst;

    type ExprOut = ();

    fn trav_program(&mut self, program: &mut Program<'ast, TypedAst>) {
        self.fundef_names = program.fundef_names();

        self.output.push_str(&format!("#include \"{}.h\"\n\n", self.module_name));
        self.output.push_str(HEADER);

        self.output.push('\n');
        self.output.push_str("//\n");
        self.output.push_str("// Forward declarations\n");
        self.output.push_str("//\n");

        for (_name, overloads) in &program.overloads {
            for (_sig, fundef_ids) in overloads {
                for fundef_id in fundef_ids {
                    self.output.push('\n');
                    self.emit_function_prototype(program.fundef(*fundef_id));
                }
            }
        }

        self.output.push('\n');
        self.output.push_str("//\n");
        self.output.push_str("// Wrappers\n");
        self.output.push_str("//\n");

        for (name, overloads) in &program.overloads {
            for (sig, fundef_ids) in overloads {
                if overloads.len() > 1 || fundef_ids.len() > 1 {
                    self.output.push('\n');
                    let first = program.fundef(fundef_ids[0]);
                    self.emit_wrapper_prototype(&name, sig, &first.ret_type.basetype);
                }
            }
        }

        self.output.push('\n');
        self.output.push_str("//\n");
        self.output.push_str("// Implementations\n");
        self.output.push_str("//\n");

        for (_, fundef) in program.fundefs.iter_mut() {
            self.output.push('\n');
            self.trav_fundef(fundef);
        }

        for (name, overloads) in &program.overloads {
            for (sig, fundef_ids) in overloads {
                if overloads.len() > 1 || fundef_ids.len() > 1 {
                    self.output.push('\n');
                    let family: Vec<&Fundef<TypedAst>> = fundef_ids.iter().map(|&id| program.fundef(id)).collect();
                    self.emit_wrapper_function(&name, sig, &family);
                }
            }
        }
    }

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, TypedAst>) {
        self.arg_names = fundef.args.iter().map(|arg| arg.id.clone()).collect();
        self.arg_types = fundef.args.iter().map(|arg| arg.ty.clone()).collect();
        self.ret_type = Some(fundef.ret_type.clone());
        let args: Vec<String> = fundef.args.iter()
            .map(|arg| format!("{} {}", arg.ty.ctype(), arg.id))
            .collect();

        self.push_line(&format!(
            "{} IMP_{}({}) {{",
            fundef.ret_type.ctype(), fundef.name, args.join(", ")
        ));

        self.indent += 1;
        for assign in &mut fundef.shape_prelude {
            self.trav_assign(assign);
        }
        for stmt in &mut fundef.body.stmts {
            self.trav_stmt(stmt);
        }
        self.emit_return(fundef.body.ret);
        self.indent -= 1;

        self.push_line("}");
        self.ret_type = None;
    }

    fn trav_body(&mut self, _body: &mut Body<'ast, Self::Ast>) {
        unreachable!("needs to be implemented in a case-by-case basis")
    }

    fn trav_assign(&mut self, assign: &mut Assign<'ast, Self::Ast>) {
        let prev_lhs_target = self.lhs_target.take();
        self.lhs_target = Some((assign.lhs.name.clone(), assign.lhs.ty.clone()));

        let ty = assign.lhs.ty.clone();
        let name = assign.lhs.name.clone();

        self.trav_expr(assign.expr);
        if !matches!(&*assign.expr.borrow(), Expr::Tensor(_) | Expr::Fold(_) | Expr::Array(_)) {
            let rhs = self.expr_stack.pop().expect("expression stack underflow");
            self.push_line(&format!("{} {} = {};", ty.ctype(), name, rhs));
        }

        self.lhs_target = prev_lhs_target;
    }

    fn trav_printf(&mut self, printf: &mut Printf<'ast, Self::Ast>) {
        let id = self.nameof(&printf.id);
        self.push_line(&format!("printf(\"Hello, {}\\n\");", id));
    }

    fn trav_cond(&mut self, cond: &mut Cond<'ast, Self::Ast>) {
        if cond.then_branch.stmts.is_empty() && cond.else_branch.stmts.is_empty() {
            let c = self.nameof(&cond.cond);
            let t = self.nameof(&cond.then_branch.ret);
            let f = self.nameof(&cond.else_branch.ret);
            self.expr_stack.push(format!("{} ? {} : {}", c, t, f));
        } else {
            self.push_line(&format!("{} cond_ret;", self.id_type(&cond.then_branch.ret).ctype()));

            let c = self.nameof(&cond.cond);
            self.push_line(&format!("if ({}) {{", c));
            self.indent += 1;

            for stmt in &mut cond.then_branch.stmts {
                self.trav_stmt(stmt);
            }
            let t = self.nameof(&cond.then_branch.ret);
            self.push_line(&format!("cond_ret = {};", t));

            self.indent -= 1;
            self.push_line("} else {");
            self.indent += 1;

            for stmt in &mut cond.else_branch.stmts {
                self.trav_stmt(stmt);
            }
            let f = self.nameof(&cond.else_branch.ret);
            self.push_line(&format!("cond_ret = {};", f));

            self.indent -= 1;
            self.push_line("}");

            self.expr_stack.push("cond_ret".to_string());
        }
    }

    fn trav_tensor(&mut self, tensor: &mut Tensor<'ast, Self::Ast>) {
        let (target_name, target_ty) = self.lhs_target.clone().expect("tensor target must be set");
        let base = target_ty.basetype.ctype();
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

        // Heap-allocate the result shape array.
        self.push_line(&format!("size_t *{shp_name} = IMP_ALLOC_SHAPE_VEC({rank});"));
        for d in 0..rank {
            if tensor.lb.is_some() {
                self.push_line(&format!("{shp_name}[{d}] = {iv_name}_ub{d}_{t_uid} - {iv_name}_lb{d}_{t_uid};"));
            } else {
                self.push_line(&format!("{shp_name}[{d}] = {iv_name}_ub{d}_{t_uid};"));
            }
        }

        self.push_line(&format!("{base} *{data_name} = IMP_ALLOC_DATA({base}, {len_name});"));

        // Generate k nested for-loops.
        for d in 0..rank {
            if tensor.lb.is_some() {
                self.push_line(&format!("for (size_t {iv_name}_{d}_{t_uid} = {iv_name}_lb{d}_{t_uid}; {iv_name}_{d}_{t_uid} < {iv_name}_ub{d}_{t_uid}; {iv_name}_{d}_{t_uid}++) {{"));
            } else {
                self.push_line(&format!("for (size_t {iv_name}_{d}_{t_uid} = 0; {iv_name}_{d}_{t_uid} < {iv_name}_ub{d}_{t_uid}; {iv_name}_{d}_{t_uid}++) {{"));
            }
            self.indent += 1;
        }

        // Build iv as a stack-allocated ImpArrayRaw so that iv[i] selections work.
        let iv_elem = &tensor.iv.ty.basetype.ctype();
        let iv_components: Vec<String> = (0..rank)
            .map(|d| format!("({iv_elem}){iv_name}_{d}_{t_uid}"))
            .collect();
        self.push_line(&format!(
            "{iv_elem} {iv_name}_data_{t_uid}[{rank}] = {{ {} }};",
            iv_components.join(", ")
        ));
        self.push_line(&format!("size_t {iv_name}_shp_arr_{t_uid}[1] = {{ {rank} }};"));
        self.push_line(&format!("ImpArrayRaw {iv_name} = IMP_MK_ARRAY({rank}, 1, {iv_name}_shp_arr_{t_uid}, {iv_name}_data_{t_uid});"));

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
        for stmt in &mut tensor.body.stmts {
            self.trav_stmt(stmt);
        }

        // Store element into the flat result buffer.
        let mut ret = self.render_id(tensor.body.ret);
        if rank == 1 && ret == iv_name {
            ret = format!("(({iv_elem}*){iv_name}.data)[0]");
        }
        self.push_line(&format!("{data_name}[{iv_name}_flat] = {ret};"));

        // Close nested loops.
        for _ in 0..rank {
            self.indent -= 1;
            self.push_line("}");
        }

        self.push_line(&format!("ImpArrayRaw {target_name} = IMP_MK_ARRAY({len_name}, {rank}, {shp_name}, {data_name});"));
    }

    fn trav_fold(&mut self, fold: &mut Fold<'ast, Self::Ast>) {
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

        let neutral_expr = self.render_id(fold.neutral);
        self.push_line(&format!("{} {} = {};", target_ty.ctype(), target_name, neutral_expr));

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
                self.push_line(&format!("for (size_t {iv_name}_{d}_{t_uid} = {iv_name}_lb{d}_{t_uid}; {iv_name}_{d}_{t_uid} < {iv_name}_ub{d}_{t_uid}; {iv_name}_{d}_{t_uid}++) {{"));
            } else {
                self.push_line(&format!("for (size_t {iv_name}_{d}_{t_uid} = 0; {iv_name}_{d}_{t_uid} < {iv_name}_ub{d}_{t_uid}; {iv_name}_{d}_{t_uid}++) {{"));
            }
            self.indent += 1;
        }

        let iv_elem = fold.selection.iv.ty.basetype.ctype();
        let iv_components: Vec<String> = (0..rank)
            .map(|d| format!("({iv_elem}){iv_name}_{d}_{t_uid}"))
            .collect();
        self.push_line(&format!(
            "{iv_elem} {iv_name}_data_{t_uid}[{rank}] = {{ {} }};",
            iv_components.join(", ")
        ));
        self.push_line(&format!("size_t {iv_name}_shp_arr_{t_uid}[1] = {{ {rank} }};"));
        self.push_line(&format!("ImpArrayRaw {iv_name} = IMP_MK_ARRAY({rank}, 1, {iv_name}_shp_arr_{t_uid}, {iv_name}_data_{t_uid});"));

        for stmt in &mut fold.selection.body.stmts {
            self.trav_stmt(stmt);
        }

        let sel_expr = self.render_id(fold.selection.body.ret);

        let (fold_name, call_args) = match &fold.foldfun {
            FoldFun::Name(id) => {
                let name = self.fundef_names[id.index()].clone();
                (name, vec![target_name.clone(), sel_expr])
            }
            FoldFun::Apply { id, args } => {
                let name = self.fundef_names[id.index()].clone();
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
                            out.push(self.render_id(bound.clone()));
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

    fn trav_call(&mut self, call: &mut Call<'ast, TypedAst>) {
        let name = self.fundef_names[call.id.index()].clone();

        let args: Vec<String> = call.args.iter()
            .map(|arg| self.render_id(*arg))
            .collect();
        self.expr_stack.push(format!("IMP_{}({})", name, args.join(", ")));
    }

    fn trav_prf(&mut self, prf: &mut Prf<'ast, TypedAst>) {
        use Prf::*;
        let rendered = match &prf {
            DimA(arr) => {
                let arg = self.render_id(*arr);
                format!("{arg}.dim")
            }
            ShapeA(arr) => {
                let arg = self.render_id(*arr);
                self.shp_uid += 1;
                let uid = self.shp_uid;
                let meta = format!("_shp{uid}_meta");
                let data = format!("_shp{uid}_data");
                let wrap = format!("_shp{uid}");
                self.push_line(&format!("size_t *{meta} = (size_t *)malloc(sizeof(size_t));"));
                self.push_line(&format!("*{meta} = {arg}.dim;"));
                self.push_line(&format!("size_t *{data} = IMP_ALLOC_SHAPE_VEC({arg}.dim);"));
                self.push_line(&format!("for (size_t _i = 0; _i < {arg}.dim; _i++) {{ {data}[_i] = {arg}.shp[_i]; }}"));
                self.push_line(&format!("ImpArrayRaw {wrap} = IMP_MK_ARRAY({arg}.dim, 1, {meta}, {data});"));
                wrap
            }
            SelVxA(idx, arr) => {
                let arr_name = self.render_id(*arr);
                let idx_name = self.render_id(*idx);
                let elem_base = elem_ctype_of_id(arr);
                format!("(({elem_base} *){arr_name}.data)[imp_flat_index({arr_name}, {idx_name})]")
            }
            AddSxS(a, b) => format!("{} + {}", self.render_id(*a), self.render_id(*b)),
            SubSxS(a, b) => format!("{} - {}", self.render_id(*a), self.render_id(*b)),
            MulSxS(a, b) => format!("{} * {}", self.render_id(*a), self.render_id(*b)),
            DivSxS(a, b) => format!("{} / {}", self.render_id(*a), self.render_id(*b)),
            LtSxS(a, b) => format!("{} < {}", self.render_id(*a), self.render_id(*b)),
            LeSxS(a, b) => format!("{} <= {}", self.render_id(*a), self.render_id(*b)),
            GtSxS(a, b) => format!("{} > {}", self.render_id(*a), self.render_id(*b)),
            GeSxS(a, b) => format!("{} >= {}", self.render_id(*a), self.render_id(*b)),
            EqSxS(a, b) => format!("{} == {}", self.render_id(*a), self.render_id(*b)),
            NeSxS(a, b) => format!("{} != {}", self.render_id(*a), self.render_id(*b)),
            NegS(a) => format!("-{}", self.render_id(*a)),
            NotS(a) => format!("!{}", self.render_id(*a)),
        };

        self.expr_stack.push(rendered);
    }

    fn trav_array(&mut self, array: &mut Array<'ast, Self::Ast>) {
        let (target_name, target_ty) = self.lhs_target.clone().expect("array target must be set");
        let data_name = format!("{}_data", target_name);
        let shp_name = format!("{}_shp", target_name);
        let len_name = format!("{}_len", target_name);
        let base = target_ty.basetype.ctype();

        self.push_line(&format!("size_t {} = {};", len_name, array.elems.len()));
        self.push_line(&format!("{base} *{data_name} = IMP_ALLOC_DATA({base}, {len_name});"));

        for (i, value) in array.elems.iter().enumerate() {
            let rendered = self.render_id(*value);
            self.push_line(&format!("{data_name}[{i}] = {rendered};"));
        }

        self.push_line(&format!("size_t *{shp_name} = IMP_ALLOC_SHAPE_VEC(1);"));
        self.push_line(&format!("{shp_name}[0] = {len_name};"));
        self.push_line(&format!("ImpArrayRaw {target_name} = IMP_MK_ARRAY({len_name}, 1, {shp_name}, {data_name});"));
    }

    fn trav_id(&mut self, id: &mut Id<'ast, Self::Ast>) {
        match id {
            Id::Arg(i) => self.expr_stack.push(self.arg_names[*i].clone()),
            Id::Var(lvis) => self.expr_stack.push(lvis.name.clone()),
        }
    }

    fn trav_const(&mut self, c: &mut Const) {
        self.expr_stack.push(c.to_string())
    }
}

fn shape_match_condition(shape: &TypePattern, arg: &str) -> String {
    match shape {
        TypePattern::Scalar => {
            format!("{arg}.dim == 0")
        }
        TypePattern::Axes(axes) => {
            if axes.iter().any(|ax| matches!(ax, AxisPattern::Rank(_))) {
                // rank-polymorphic array: any non-zero dim
                return format!("{arg}.dim > 0");
            }

            let mut checks = vec![
                format!("{arg}.dim == {}", axes.len()),
            ];
            for (i, axis) in axes.iter().enumerate() {
                if let AxisPattern::Dim(DimCapture::Known(v)) = axis {
                    checks.push(format!("{arg}.shp[{i}] == {v}"));
                }
            }
            checks.join(" && ")
        }
    }
}

fn wrapper_call_arg(shape: &TypePattern, arg: &str, base: &BaseType) -> String {
    match shape {
        TypePattern::Scalar => format!("(*({}*){}.data)", base.ctype(), arg),
        TypePattern::Axes(_) => arg.to_owned(),
    }
}

fn elem_ctype_of_id(id: &Id<'_, TypedAst>) -> String {
    match id {
        Id::Arg(_) => "uint32_t".to_string(),
        Id::Var(v) => v.ty.basetype.ctype(),
    }
}
