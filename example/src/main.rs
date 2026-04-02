include!(concat!(env!("OUT_DIR"), "/simple.rs"));

fn main() {
    println!("shouldbefolded = {}", shouldbefolded());
    println!("addthem = {}", addthem(4, 2, 3));
    assert_eq!(shouldbefolded(), addthem(4, 2, 3));
}
