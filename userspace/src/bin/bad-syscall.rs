use core::arch::asm;

fn main() {
    unsafe {
        asm!(
            "li a7, 999",
            "ecall",
            out("a7") _,
            out("a0") _,
        );
    }
    println!("BUG: should have been killed");
}
