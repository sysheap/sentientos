use std::env;

fn main() {
    for (index, arg) in env::args().skip(1).enumerate() {
        if index > 0 {
            print!(" ");
        }
        print!("{arg}");
    }
    println!();
}
