use super::*;

#[derive(Clone, Debug)]
pub struct TraitDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub args: Vec<PolyType>,
    pub ret: PolyType,
}

#[derive(Clone, Debug)]
pub struct PolyType {
    pub head: String,
    pub shape: Option<ShapePattern>,
}
