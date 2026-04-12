#![allow(dead_code)]
#![allow(unused_parens)]
include!(concat!(env!("OUT_DIR"), "/IMPstdlib.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use imp_core::*;
    use parameterized::parameterized;

    #[parameterized(n = { 0, 1, 10 })]
    fn test_iota(n: usize) {
        let arr = expect_array(iota(n));
        assert_eq!(arr.shp, vec![n]);
        assert_eq!(arr.data, (0..n).collect::<Vec<_>>());
    }
}
