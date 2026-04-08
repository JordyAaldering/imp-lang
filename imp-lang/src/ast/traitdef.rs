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
    pub type_params: Vec<String>,
    pub args: Vec<PolyType>,
    pub ret: PolyType,
}

#[derive(Clone, Debug)]
pub struct MemberBound {
    pub type_var: String,
    pub type_set: String,
}

#[derive(Clone, Debug)]
pub enum WhereBound {
    Member(MemberBound),
}

#[derive(Clone, Debug)]
pub struct MemberDef {
    pub type_name: String,
    pub member: PolyType,
}

#[derive(Clone, Debug)]
pub struct ImplDef {
    pub trait_name: String,
    pub args: Vec<PolyType>,
    pub ret_type: PolyType,
    pub type_params: Vec<String>,
    pub where_bounds: Vec<WhereBound>,
    pub methods: Vec<TraitMethodSig>,
}

impl PolyType {
    pub fn from_concrete(ty: &Type) -> Self {
        let head = match ty.ty {
            BaseType::I32 => "i32",
            BaseType::I64 => "i64",
            BaseType::U32 => "u32",
            BaseType::U64 => "u64",
            BaseType::Usize => "usize",
            BaseType::F32 => "f32",
            BaseType::F64 => "f64",
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