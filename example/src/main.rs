#![allow(dead_code)]
#![allow(unused_parens)]
include!(concat!(env!("OUT_DIR"), "/IMPsimple.rs"));

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

    let cat = expect_array(cat(expect_array(iota(3)), expect_array(iota(4))));
    println!("cat = {:?}", cat.data);

    let fold_input = ImpArray { shp: vec![4], data: vec![1i32, 2, 3, 4] };
    let fold_sum = expect_scalar(sum(fold_input));
    assert_eq!(fold_sum, 10);
    println!("sum = {}", fold_sum);

    let fold2d_input = ImpArray {
        shp: vec![2, 3],
        data: vec![1i32, 2, 3, 4, 5, 6],
    };
    let fold2d_sum = expect_scalar(sum(fold2d_input));
    //assert_eq!(fold2d_sum, 21);
    println!("sum2d = {}", fold2d_sum);

    let fold2d_input = ImpArray {
        shp: vec![3,2],
        data: vec![1i32, 2, 3, 4, 5, 6],
    };
    let fold2d_sum = expect_scalar(sum(fold2d_input));
    //assert_eq!(fold2d_sum, 21);
    println!("sum2d = {}", fold2d_sum);

    let fold_last_input = ImpArray {
        shp: vec![2, 3],
        data: vec![1i32, 2, 3, 4, 5, 6],
    };
    let fold_last = expect_array(sumlast(fold_last_input));
    assert_eq!(fold_last.shp, vec![3]);
    println!("sumlast = {:?}", fold_last.data);

    let ub: usize = 10;
    let arr: ImpArray<usize> = expect_array(iota(ub));
    assert_eq!(arr.shp, vec![ub]);
    assert_eq!(arr.data, (0..ub).collect::<Vec<usize>>());
    println!("arr.data = {:?}", arr.data);

    let arr1: ImpArray<usize> = expect_array(iota(15));
    let arr2: ImpArray<usize> = expect_array(iota(15));
    let res: ImpArray<usize> = expect_array(my_add_after_iota(arr1, arr2));
    println!("iota + iota = {:?}", res.data);

    let overldemo = expect_scalar(overload_demo_usize_usize(ImpArrayOrScalar::Scalar(4usize), ImpArrayOrScalar::Scalar(5usize)));
    println!("overload_demo scalar = {:?}", overldemo);

    // Obviously, we should not have to write 'ovl' (overload).
    // We should generate each variant with a unique name, and then a wrapper with the original
    // name that dispatches to the correct variant based on argument types and shapes
    let overldemo: ImpArray<usize> = expect_array(overload_demo_usize_usize(ImpArrayOrScalar::Array(expect_array(four())), ImpArrayOrScalar::Array(expect_array(four()))));
    println!("overload_demo vector = {:?}", overldemo.data);

    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let add_demo_mismatch = std::panic::catch_unwind(|| add_demo(expect_array(four()), expect_array(five())));
    std::panic::set_hook(panic_hook);
    assert!(add_demo_mismatch.is_err());
    println!("add_demo mismatched extents rejected in Rust FFI wrapper");

    let shp: ImpArray<usize> = expect_array(shape(arr));
    println!("shape(arr) = {:?}", shp.data);

    let arr2: ImpArray<u32> = expect_array(arrays());
    assert_eq!(arr2.shp, vec![5]);
    println!("arr2.data = {:?}", arr2.data);

    println!("sel = {}", expect_scalar(sel_demo()));

    println!("scalar_add_demo = {}", expect_scalar(scalar_add_demo()));

    let dyn_sum = add_dyn(expect_array(iota(4)), expect_array(iota(4)));
    println!("add_dyn = {:?}", dyn_sum);

    // double free detected
    // let arr = scalar_or_array(ImpArrayOrScalar::Array(ImpArray { shp: vec![6], data: vec![1,2,3,4,5,6] }));
    // println!("scalar_or_array = {:?}", arr);
}
