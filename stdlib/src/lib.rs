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
        let imp_shp = ImpArray { shp: vec![shp.len()], data: shp.clone() };
        let arr = genarray_usize_i32(imp_shp, val);
        assert_eq!(arr.shp, shp);
        assert_eq!(arr.data, vec![val; arr.data.len()]);
    }

    #[parameterized(
        shp = { vec![0], vec![1], vec![10] },
        val = { 37, 0, 42 },
    )]
    fn test_genarray_usize(shp: Vec<usize>, val: usize) {
        let imp_shp = ImpArray { shp: vec![shp.len()], data: shp.clone() };
        let arr = genarray_usize_usize(imp_shp, val);
        assert_eq!(arr.shp, shp);
        assert_eq!(arr.data, vec![val; arr.data.len()]);

    }

    #[parameterized(n = { 0, 1, 10 })]
    fn test_iota(n: usize) {
        let arr = iota(n);
        assert_eq!(arr.shp, vec![n]);
        assert_eq!(arr.data, (0..n).collect::<Vec<_>>());
    }

    #[parameterized(n = { 0, 1, 10 })]
    fn test_scalar_add(n: usize) {
        let arr = iota(n);
        let one = ImpArray::scalar(1usize);
        let arr = add_usize_usize(arr, one);
        assert_eq!(arr.shp, vec![n]);
        assert_eq!(arr.data, (1..=n).collect::<Vec<_>>());
    }

    #[parameterized(n = { 0, 1, 10 })]
    fn test_sum(n: usize) {
        let arr = iota(n);
        let v = sum_usize(arr);
        assert_eq!(v, (0..n).sum());
    }

    #[parameterized(n = { 0, 1, 10 })]
    fn test_prod(n: usize) {
        let arr = iota(n);
        let one = ImpArray::scalar(1usize);
        let arr = add_usize_usize(arr, one);
        let v = sum_usize(arr);
        assert_eq!(v, (1..=n).sum());
    }
}
