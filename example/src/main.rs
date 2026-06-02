#![allow(dead_code)]
#![allow(unused_parens)]
include!(concat!(env!("OUT_DIR"), "/IMPsimple.rs"));

use imp_core::*;

fn main() {
    let folded = shouldbefolded();
    println!("shouldbefolded = {}", folded);
    assert_eq!(folded, 9);

    let cat = cat(iota(3), iota(4));
    println!("cat = {:?}", cat.data);

    let fold_input = ImpArray { shp: vec![4], data: vec![1i32, 2, 3, 4] };
    let fold_sum = sum(fold_input);
    assert_eq!(fold_sum, 10);
    println!("sum = {}", fold_sum);

    let fold2d_input = ImpArray {
        shp: vec![2, 3],
        data: vec![1i32, 2, 3, 4, 5, 6],
    };
    let fold2d_sum = sum(fold2d_input);
    //assert_eq!(fold2d_sum, 21);
    println!("sum2d = {}", fold2d_sum);

    let fold2d_input = ImpArray {
        shp: vec![3,2],
        data: vec![1i32, 2, 3, 4, 5, 6],
    };
    let fold2d_sum = sum(fold2d_input);
    //assert_eq!(fold2d_sum, 21);
    println!("sum2d = {}", fold2d_sum);

    let fold_last_input = ImpArray {
        shp: vec![2, 3],
        data: vec![1i32, 2, 3, 4, 5, 6],
    };
    let fold_last = sumlast(fold_last_input);
    assert_eq!(fold_last.shp, vec![3]);
    println!("sumlast = {:?}", fold_last.data);

    let ub: usize = 10;
    let arr: ImpArray<usize> = iota(ub);
    assert_eq!(arr.shp, vec![ub]);
    assert_eq!(arr.data, (0..ub).collect::<Vec<usize>>());
    println!("arr.data = {:?}", arr.data);

    let arr1: ImpArray<usize> = iota(15);
    let arr2: ImpArray<usize> = iota(15);
    let res: ImpArray<usize> = my_add_after_iota(arr1, arr2);
    println!("iota + iota = {:?}", res.data);

    let overldemo_arr = overload_demo_usize_usize(ImpArray::scalar(4usize), ImpArray::scalar(5usize));
    let overldemo = overldemo_arr.scalar_value();
    println!("overload_demo scalar = {:?}", overldemo);

    // Obviously, we should not have to write 'ovl' (overload).
    // We should generate each variant with a unique name, and then a wrapper with the original
    // name that dispatches to the correct variant based on argument types and shapes
    let overldemo: ImpArray<usize> = overload_demo_usize_usize(four(), four());
    println!("overload_demo vector = {:?}", overldemo.data);

    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let add_demo_mismatch = std::panic::catch_unwind(|| add_demo(four(), five()));
    std::panic::set_hook(panic_hook);
    assert!(add_demo_mismatch.is_err());
    println!("add_demo mismatched extents rejected in Rust FFI wrapper");

    let shp: ImpArray<usize> = shape(arr);
    println!("shape(arr) = {:?}", shp.data);

    let arr2: ImpArray<u32> = arrays();
    assert_eq!(arr2.shp, vec![5]);
    println!("arr2.data = {:?}", arr2.data);

    println!("sel = {}", sel_demo());

    println!("scalar_add_demo = {}", scalar_add_demo());

    let dyn_sum = add_dyn(iota(4), iota(4));
    println!("add_dyn = {:?}", dyn_sum);
}
