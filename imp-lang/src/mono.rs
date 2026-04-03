use std::collections::HashSet;

use crate::ast::*;

pub fn monomorphise_generics<'ast>(mut program: Program<'ast, ParsedAst>) -> Program<'ast, ParsedAst> {
    let generic_items: Vec<GenericFundef<'ast, ParsedAst>> = program.generic_functions.values().cloned().collect();

    for generic in generic_items {
        let Some(bound) = generic.where_bounds.iter().find(|b| b.ty_name == generic.type_param) else {
            continue;
        };

        let mut seen_types = HashSet::new();
        for impl_def in &program.impls {
            if impl_def.trait_name != bound.trait_name {
                continue;
            }

            let Some(concrete_ty) = poly_to_concrete_type(&impl_def.for_type) else {
                continue;
            };

            let type_key = format!("{}:{}", impl_def.for_type.head, shape_key(&concrete_ty.shape));
            if !seen_types.insert(type_key) {
                continue;
            }

            let mut args = Vec::with_capacity(generic.args.len());
            let mut ok = true;
            for arg in &generic.args {
                let Some(arg_ty) = instantiate_poly_type(&arg.ty, &generic.type_param, &impl_def.for_type) else {
                    ok = false;
                    break;
                };
                let farg: &'ast Farg = Box::leak(Box::new(Farg {
                    name: arg.name.clone(),
                    ty: arg_ty,
                }));
                args.push(farg);
            }
            if !ok {
                continue;
            }

            let Some(ret_ty) = instantiate_poly_type(&generic.ret_type, &generic.type_param, &impl_def.for_type) else {
                continue;
            };

            let mut key = format!("{}__{}", generic.name, mangle_poly_type(&impl_def.for_type));
            let mut public_name = key.clone();
            if impl_def.for_type.head == "u32" {
                key = generic.name.clone();
                public_name = generic.name.clone();
            }

            let instantiated = Fundef {
                name: public_name,
                ret_type: ret_ty,
                args,
                decs: generic.decs.clone(),
                body: generic.body.clone(),
            };

            program.functions.insert(key, instantiated);
        }
    }

    // Generic declarations are consumed once concrete instances are emitted.
    program.generic_functions.clear();

    program
}

fn poly_to_concrete_type(poly: &PolyType) -> Option<Type> {
    let base = match poly.head.as_str() {
        "u32" => BaseType::U32,
        "usize" => BaseType::Usize,
        "bool" => BaseType::Bool,
        _ => return None,
    };

    let shape = poly.shape.clone().unwrap_or(ShapePattern::Scalar);
    let knowledge = match &shape {
        ShapePattern::Scalar => TypeKnowledge::Scalar,
        ShapePattern::Any => TypeKnowledge::AUD,
        ShapePattern::Axes(axes) => {
            if axes.iter().any(|a| matches!(a, AxisPattern::Rank(_))) {
                let min_rank = axes.iter().filter(|a| matches!(a, AxisPattern::Dim(_))).count() as u8;
                TypeKnowledge::AUDGN { min_rank }
            } else if axes.iter().any(|a| matches!(a, AxisPattern::Dim(DimPattern::Any))) {
                TypeKnowledge::AKD
            } else {
                TypeKnowledge::AKS
            }
        }
    };

    Some(Type { ty: base, shape, knowledge })
}

fn instantiate_poly_type(poly: &PolyType, type_param: &str, replacement: &PolyType) -> Option<Type> {
    if poly.head == type_param {
        return poly_to_concrete_type(replacement);
    }
    poly_to_concrete_type(poly)
}

fn mangle_poly_type(poly: &PolyType) -> String {
    let mut out = poly.head.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
    if let Some(shape) = &poly.shape {
        out.push('_');
        out.push_str(&shape_key(shape));
    } else {
        out.push_str("_0");
    }
    out
}

fn shape_key(shape: &ShapePattern) -> String {
    match shape {
        ShapePattern::Scalar => "0".to_owned(),
        ShapePattern::Any => "any".to_owned(),
        ShapePattern::Axes(axes) => axes.iter().map(|axis| match axis {
            AxisPattern::Dim(DimPattern::Any) => "any".to_owned(),
            AxisPattern::Dim(DimPattern::Known(n)) => n.to_string(),
            AxisPattern::Dim(DimPattern::Var(v)) => v.name.clone(),
            AxisPattern::Rank(cap) => format!("{}_{}", cap.dim_name, cap.shp_name),
        }).collect::<Vec<_>>().join("_")
    }
}