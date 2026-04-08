use super::*;

#[derive(Clone, Debug)]
pub struct ImplDef {
    pub trait_name: String,
    pub args: Vec<PolyType>,
    pub ret_type: PolyType,
    pub type_params: Vec<String>,
    pub where_bounds: Vec<MemberBound>,
    pub methods: Vec<TraitMethodSig>,
}

#[derive(Clone, Debug)]
pub struct TraitMethodSig {
    pub name: String,
    pub args: Vec<PolyArg>,
    pub ret_type: PolyType,
}

#[derive(Clone, Debug)]
pub struct MemberBound {
    pub type_var: String,
    pub type_set: String,
}

#[derive(Clone, Debug)]
pub struct PolyArg {
    pub name: String,
    pub ty: PolyType,
}
