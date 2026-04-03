#[derive(Clone, Debug)]
pub struct Type {
    pub ty: BaseType,
    pub shp: Shape,
}

#[derive(Clone, Copy, Debug)]
pub enum BaseType {
    U32,
    Bool,
}

#[derive(Clone, Debug)]
pub enum Shape {
    Scalar,
    Vector(String),
}

impl Type {
    pub fn scalar(ty: BaseType) -> Self {
        Self { ty, shp: Shape::Scalar }
    }

    pub fn vector(ty: BaseType, extent: &str) -> Self {
        Self { ty, shp: Shape::Vector(extent.to_owned()) }
    }
}
