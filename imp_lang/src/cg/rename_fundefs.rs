use std::collections::HashSet;

use crate::ast::*;

pub fn rename_fundefs(program: &mut Program<'_, TypedAst>) {
    RenameFundefs::default().trav_program(program);
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
#[derive(Default)]
struct RenameFundefs {
    #[cfg(debug_assertions)]
    used_names: HashSet<String>,
}

impl<'ast> Traverse<'ast> for RenameFundefs {
    type Ast = TypedAst;

    type ExprOut = ();

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
        let arg_suffix = mangle_args(&fundef.args);

        debug_assert!(!fundef.name.ends_with(&arg_suffix), "It seems we tried to mangle function `{}' twice", fundef.name);

        fundef.name.push_str("__");
        fundef.name.push_str(&arg_suffix);

        debug_assert!(self.used_names.insert(fundef.name.clone()), "Name collision: {}", fundef.name);
    }

    fn trav_farg(&mut self, _arg: &mut Farg) {

    }
}

fn mangle_args<'a>(args: &[Farg]) -> String
{
    if args.is_empty() {
        "void".to_string()
    } else {
        args.iter()
            .map(|arg| mangle_type(&arg.ty))
            .collect::<Vec<String>>()
            .join("__")
    }
}

pub fn mangle_type(ty: &Type) -> String {
    format!("{}_{}", ty.basetype.rstype(), mangle_shape(&ty.shape))
}

fn mangle_shape(shape: &TypePattern) -> String {
    match shape {
        TypePattern::Scalar => "0".to_string(),
        TypePattern::Axes(axes) => {
            if axes.is_empty() {
                "0".to_string()
            } else {
                axes.iter()
                    .map(mangle_axis)
                    .collect::<Vec<String>>()
                    .join("_")
            }
        }
    }
}

fn mangle_axis(axis: &AxisPattern) -> String {
    match axis {
        AxisPattern::Dim(dim) => mangle_axis_dim(dim),
        AxisPattern::Rank(capture) => format!("{}_{}", mangle_axis_dim(&capture.dim), capture.shp),
    }
}

fn mangle_axis_dim(axis: &DimCapture) -> String {
    match axis {
        DimCapture::Known(v) => v.to_string(),
        DimCapture::Var(ext) => ext.clone(),
    }
}
