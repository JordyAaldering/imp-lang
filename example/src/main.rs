include!(concat!(env!("OUT_DIR"), "/simple.rs"));

use compiler::core::ImpArrayu32;

fn main() {
    println!("shouldbefolded = {}", shouldbefolded());
    println!("addthem = {}", addthem(4, 2, 3));
    assert_eq!(shouldbefolded(), addthem(4, 2, 3));

    // This does not work yet, but it should:
    let ub = 10;
    let arr: ImpArrayu32 = makevector(ub);
    assert_eq!(arr.shp, vec![ub as usize]);
    println!("arr.data = {:?}", arr.data);
}
