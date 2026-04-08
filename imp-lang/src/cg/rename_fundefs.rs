use std::collections::HashSet;

use crate::ast::*;

/// Functions may be overloaded, e.g.
///
/// ```
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
/// ```
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
pub struct RenameFundefs;

pub fn rename_fundefs<'ast>(program: &mut Program<'ast, TypedAst>) {
    let pass = RenameFundefs;
    pass.rename(program);
}

impl RenameFundefs {
    fn rename<'ast>(&self, program: &mut Program<'ast, TypedAst>) {
    let mut used_names = HashSet::new();

    let mut function_names: Vec<String> = program.functions.keys().cloned().collect();
    function_names.sort();

    for function_name in function_names {
        let Some(fundef) = program.functions.get_mut(&function_name) else {
            continue;
        };

        let base_name = mangle_fundef_name(&function_name, &fundef.args);
        let unique_name = make_unique(base_name, &mut used_names);
        fundef.name = unique_name;
    }
}
}

fn make_unique(mut candidate: String, used_names: &mut HashSet<String>) -> String {
    if used_names.insert(candidate.clone()) {
        return candidate;
    }

    let root = candidate;
    let mut i = 1usize;
    loop {
        candidate = format!("{}__alt{}", root, i);
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
        i += 1;
    }
}

pub fn mangle_fundef_name(base_name: &str, args: &[Farg]) -> String {
    format!("{}__{}", base_name, mangle_arg_types(args.iter().map(|arg| &arg.ty)))
}

pub fn mangle_call_name(base_name: &str, arg_types: &[Type]) -> String {
    format!("{}__{}", base_name, mangle_arg_types(arg_types.iter()))
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
            DimPattern::Var(ext) => ext.name.clone(),
        },
        AxisPattern::Rank(capture) => format!("{}_{}", capture.dim_name, capture.shp_name),
    }
}
