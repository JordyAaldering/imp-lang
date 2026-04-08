include!(concat!(env!("OUT_DIR"), "/simple.rs"));

use imp_core::*;

fn expect_scalar<T: Copy>(value: ImpArrayOrScalar<T>) -> T {
    match value {
        ImpArrayOrScalar::Scalar(v) => v,
        ImpArrayOrScalar::Array(_) => panic!("expected scalar result"),
    }
}

fn expect_array<T: Copy>(value: ImpArrayOrScalar<T>) -> ImpArray<T> {
    match value {
        ImpArrayOrScalar::Array(v) => v,
        ImpArrayOrScalar::Scalar(_) => panic!("expected array result"),
    }
}

fn main() {
    let folded = expect_scalar(shouldbefolded());
    println!("shouldbefolded = {}", folded);
    assert_eq!(folded, 9);

    let ub: usize = 10;
    let arr: ImpArray<usize> = expect_array(iota(ub));
    assert_eq!(arr.shp, vec![ub]);
    assert_eq!(arr.data, (0..ub).collect::<Vec<usize>>());
    println!("arr.data = {:?}", arr.data);

    let arr1: ImpArray<usize> = expect_array(iota(15));
    let arr2: ImpArray<usize> = expect_array(iota(15));
    let res: ImpArray<usize> = expect_array(my_add_after_iota(arr1, arr2));
    println!("iota + iota = {:?}", res.data);

    let shp: ImpArray<usize> = expect_array(shape(arr));
    println!("shape(arr) = {:?}", shp.data);

    let arr2: ImpArray<u32> = expect_array(arrays());
    assert_eq!(arr2.shp, vec![5]);
    println!("arr2.data = {:?}", arr2.data);

    println!("sel = {}", expect_scalar(sel()));

    let arr = scalar_or_array(ImpArrayOrScalar::Scalar(37));
    println!("scalar_or_array = {:?}", arr);

    // double free detected
    // let arr = scalar_or_array(ImpArrayOrScalar::Array(ImpArray { shp: vec![6], data: vec![1,2,3,4,5,6] }));
    // println!("scalar_or_array = {:?}", arr);
}
