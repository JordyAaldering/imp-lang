use std::collections::HashSet;

use crate::ast::*;

pub fn rename_fundefs<'ast>(program: &mut Program<'ast, TypedAst>) {
    RenameFundefs::new().trav_program(program);
}

/// Functions may be overloaded, e.g.
///
/// ```imp
/// foo(u32 x) -> u32
///
/// foo(u32[n] x) -> u32[n]
///
/// foo(u32[n] x, u32[n] y) -> u32[n]
///
/// foo(u32[n,m] x) -> u32[n,m]
/// ```
///
/// The number of return values must stay the same.
/// It is not possible to dispatch based on the return type.
///
/// C does not have overloading, so we need a way to consistently rename functions.
/// For this, we append each argument type to the end of the function's name.
///
/// Not only the base type, but also the type pattern.
///
/// ```imp
/// foo__u32_0(u32 x) -> u32
///
/// foo__u32_n(u32[n] x) -> u32[n]
///
/// foo__u32_n__u32_n(u32[n] x, u32[n] y) -> u32[n]
///
/// foo__u32_n__u32_m(u32[n] x, u32[m] y) -> u32[n]
///
/// foo__u32_n_m(u32[n,m] x) -> u32[n,m]
/// ```
///
/// Although in SaC foo__u32_n__u32_n and foo__u32_n__u32_m would be considered the same, we do allow it here.
/// This is not possible in general: crucially, it requires some ordering in the functions.
/// Here, foo__u32_n__u32_n is a more specific overload of foo__u32_n__u32_m.
/// Thus, foo__u32_n__u32_n < foo__u32_n__u32_m
///
/// For example, this is not allowed for bar(u32[o:oshp,i:ishp] a, u32[o:oshp] b) and bar(u32[o:oshp] a, u32[o:osho,i:ishp] b).
/// As, in the case where the shapes of a and b are the same, and thus i == 0, both overloads would be equally specific.
/// Namely, there must be a clear ordering
pub struct RenameFundefs {
    #[cfg(debug_assertions)]
    used_names: HashSet<String>,
}

impl RenameFundefs {
    pub fn new() -> Self {
        Self {
            #[cfg(debug_assertions)]
            used_names: HashSet::new(),
        }
    }
}

impl<'ast> Traverse<'ast> for RenameFundefs {
    type Ast = TypedAst;

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        let old = fundef.name.clone();
        fundef.name = mangle_fundef_name(&fundef.name, &fundef.args);

        eprintln!("rename {} to {}", old, fundef.name);

        #[cfg(debug_assertions)]
        if !self.used_names.insert(fundef.name.clone()) {
            panic!("name collision: {}", fundef.name);
        }
    }
}

pub fn mangle_fundef_name(base_name: &str, args: &[Farg]) -> String {
    let base_name = sanitize_symbol_name(base_name);
    let arg_suffix = mangle_arg_types(args.iter().map(|arg| &arg.ty));
    if base_name.ends_with(&format!("__{arg_suffix}")) {
        return base_name;
    }
    format!("{}__{}", base_name, arg_suffix)
}

pub fn mangle_call_name(base_name: &str, arg_types: &[Type]) -> String {
    format!("{}__{}", sanitize_symbol_name(base_name), mangle_arg_types(arg_types.iter()))
}

pub fn sanitize_symbol_name(name: &str) -> String {
    name.strip_prefix('@').unwrap_or(name).to_owned()
}

fn mangle_arg_types<'a, I>(arg_types: I) -> String
where
    I: Iterator<Item = &'a Type>,
{
    let parts: Vec<String> = arg_types.map(mangle_type).collect();
    if parts.is_empty() {
        "void".to_owned()
    } else {
        parts.join("__")
    }
}

pub fn mangle_type(ty: &Type) -> String {
    use BaseType::*;
    let base = match &ty.ty {
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
    format!("{}_{}", base, mangle_shape(&ty.shape))
}

fn mangle_shape(shape: &TypePattern) -> String {
    match shape {
        TypePattern::Scalar => "0".to_owned(),
        TypePattern::Any => "any".to_owned(),
        TypePattern::Axes(axes) => {
            if axes.is_empty() {
                return "0".to_owned();
            }
            axes.iter().map(mangle_axis).collect::<Vec<_>>().join("_")
        }
    }
}

fn mangle_axis(axis: &AxisPattern) -> String {
    match axis {
        AxisPattern::Dim(dim) => match dim {
            DimPattern::Any => "any".to_owned(),
            DimPattern::Known(v) => v.to_string(),
            DimPattern::Var(ext) => ext.clone(),
        },
        AxisPattern::Rank(capture) => format!("{}_{}", capture.dim_name, capture.shp_name),
    }
}
