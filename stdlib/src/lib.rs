#![allow(dead_code)]
#![allow(unused_parens)]
include!(concat!(env!("OUT_DIR"), "/IMPstdlib.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use imp_core::*;
    use parameterized::parameterized;

    #[parameterized(
        shp = { vec![0], vec![1], vec![10] },
        val = { -37, 0, 42 },
    )]
    fn test_genarray_i32(shp: Vec<usize>, val: i32) {
        let imp_shp = ImpArrayOrScalar::Array(ImpArray { shp: vec![shp.len()], data: shp.clone() });
        let imp_val = ImpArrayOrScalar::Scalar(val);

        let arr = expect_array(genarray_usize_i32(imp_shp, imp_val));

        assert_eq!(arr.shp, shp);
        assert_eq!(arr.data, vec![val; arr.data.len()]);

    }

    #[parameterized(
        shp = { vec![0], vec![1], vec![10] },
        val = { 37, 0, 42 },
    )]
    fn test_genarray_usize(shp: Vec<usize>, val: usize) {
        let imp_shp = ImpArrayOrScalar::Array(ImpArray { shp: vec![shp.len()], data: shp.clone() });
        let imp_val = ImpArrayOrScalar::Scalar(val);

        let arr = expect_array(genarray_usize_usize(imp_shp, imp_val));

        assert_eq!(arr.shp, shp);
        assert_eq!(arr.data, vec![val; arr.data.len()]);

    }

    #[parameterized(n = { 0, 1, 10 })]
    fn test_iota(n: usize) {
        let arr = expect_array(iota(n));

        assert_eq!(arr.shp, vec![n]);
        assert_eq!(arr.data, (0..n).collect::<Vec<_>>());
    }
}
