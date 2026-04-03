include!(concat!(env!("OUT_DIR"), "/simple.rs"));

use imp_core::*;

fn main() {
    println!("shouldbefolded = {}", shouldbefolded());
    println!("addthem = {}", addthem(4, 2, 3));
    assert_eq!(shouldbefolded(), addthem(4, 2, 3));

    let ub = 10;
    let arr: ImpArray<usize> = iota(ub);
    assert_eq!(arr.shp, vec![ub]);
    assert_eq!(arr.data, (0..ub).collect::<Vec<usize>>());
    println!("arr.data = {:?}", arr.data);

    let arr2: ImpArray<u32> = arrays();
    assert_eq!(arr2.shp, vec![5]);
    println!("arr2.data = {:?}", arr2.data);

    println!("sel = {}", sel());
}
