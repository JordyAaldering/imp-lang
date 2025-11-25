include!(concat!(env!("OUT_DIR"), "/simple.rs"));

fn main() {
    println!("Hello, world! {}", addthem(4, 2, 3));
}
