use super::*;

#[derive(Clone, Debug)]
pub struct PolyType {
    pub head: String,
    pub shape: Option<ShapePattern>,
}

#[derive(Clone, Debug)]
pub struct PolyArg {
    pub name: String,
    pub ty: PolyType,
}

#[derive(Clone, Debug)]
pub struct TraitMethodSig {
    pub name: String,
    pub args: Vec<PolyArg>,
    pub ret_type: PolyType,
}

#[derive(Clone, Debug)]
pub struct TraitDef {
    pub name: String,
    pub param: String,
    pub methods: Vec<TraitMethodSig>,
}

#[derive(Clone, Debug)]
pub struct TraitBound {
    pub ty_name: String,
    pub trait_name: String,
}

#[derive(Clone, Debug)]
pub struct ImplDef {
    pub trait_name: String,
    pub for_type: PolyType,
    pub where_bounds: Vec<TraitBound>,
    pub methods: Vec<TraitMethodSig>,
}

impl PolyType {
    pub fn from_concrete(ty: &Type) -> Self {
        let head = match ty.ty {
            BaseType::U32 => "u32",
            BaseType::Usize => "usize",
            BaseType::Bool => "bool",
        };
        let shape = match &ty.shape {
            ShapePattern::Scalar => None,
            shape => Some(shape.clone()),
        };
        Self {
            head: head.to_owned(),
            shape,
        }
    }
}