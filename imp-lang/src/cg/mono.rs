use crate::ast::*;
use crate::cg::rename_fundefs;

pub fn trait_shim_name(trait_name: &str, method_name: &str, arg_types: &[Type]) -> String {
    let method = sanitize_method(method_name);
    let arg_sig = if arg_types.is_empty() {
        "void".to_owned()
    } else {
        arg_types.iter().map(rename_fundefs::mangle_type).collect::<Vec<_>>().join("__")
    };
    format!("trait_{}__{}__{}", trait_name, method, arg_sig)
}

pub fn has_impl_for_call(impls: &[ImplDef], trait_name: &str, method_name: &str, arg_types: &[Type]) -> bool {
    let _ = method_name;
    impls.iter().any(|impl_def| {
        impl_def.trait_name == trait_name
            && impl_def.args.len() == arg_types.len()
            && impl_def.args.iter().zip(arg_types.iter()).all(|(poly, ty)| poly_matches_concrete(poly, ty))
    })
}

fn sanitize_method(method_name: &str) -> String {
    method_name
        .chars()
        .map(|ch| match ch {
            '+' => "add".to_owned(),
            '-' => "sub".to_owned(),
            '*' => "mul".to_owned(),
            '/' => "div".to_owned(),
            '=' => "eq".to_owned(),
            '!' => "not".to_owned(),
            '<' => "lt".to_owned(),
            '>' => "gt".to_owned(),
            c if c.is_ascii_alphanumeric() => c.to_string(),
            _ => "_".to_owned(),
        })
        .collect::<Vec<_>>()
        .join("")
}

fn poly_matches_concrete(poly: &PolyType, ty: &Type) -> bool {
    let head_ok = match poly.head.as_str() {
        "i32" => ty.ty == BaseType::I32,
        "i64" => ty.ty == BaseType::I64,
        "u32" => ty.ty == BaseType::U32,
        "u64" => ty.ty == BaseType::U64,
        "usize" => ty.ty == BaseType::Usize,
        "f32" => ty.ty == BaseType::F32,
        "f64" => ty.ty == BaseType::F64,
        "bool" => ty.ty == BaseType::Bool,
        _ => true,
    };

    if !head_ok {
        return false;
    }

    match (&poly.shape, &ty.shape) {
        (None, ShapePattern::Scalar) => true,
        (None, _) => false,
        (Some(ShapePattern::Any), _) => true,
        (Some(ShapePattern::Scalar), ShapePattern::Scalar) => true,
        (Some(ShapePattern::Scalar), _) => false,
        (Some(ShapePattern::Axes(exp_axes)), ShapePattern::Axes(got_axes)) => {
            if exp_axes.iter().any(|a| matches!(a, AxisPattern::Rank(_))) {
                return true;
            }
            if exp_axes.len() != got_axes.len() {
                return false;
            }
            exp_axes.iter().zip(got_axes.iter()).all(|(e, g)| match (e, g) {
                (AxisPattern::Dim(DimPattern::Any), AxisPattern::Dim(_)) => true,
                (AxisPattern::Dim(DimPattern::Known(a)), AxisPattern::Dim(DimPattern::Known(b))) => a == b,
                (AxisPattern::Dim(DimPattern::Var(_)), AxisPattern::Dim(_)) => true,
                (AxisPattern::Rank(_), _) => true,
                _ => false,
            })
        }
        (Some(ShapePattern::Axes(exp_axes)), ShapePattern::Scalar) => exp_axes.is_empty(),
        (Some(ShapePattern::Axes(_)), ShapePattern::Any) => true,
    }
}
