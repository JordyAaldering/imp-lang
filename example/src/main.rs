include!(concat!(env!("OUT_DIR"), "/simple.rs"));

use compiler::core::ImpArrayu32;

fn main() {
    println!("shouldbefolded = {}", shouldbefolded());
    println!("addthem = {}", addthem(4, 2, 3));
    assert_eq!(shouldbefolded(), addthem(4, 2, 3));

    let ub = 10;
    let arr: ImpArrayu32 = iota(ub);
    assert_eq!(arr.shp, vec![ub as usize]);
    assert_eq!(arr.data, (0..ub).collect::<Vec<u32>>());
    println!("arr.data = {:?}", arr.data);
}
