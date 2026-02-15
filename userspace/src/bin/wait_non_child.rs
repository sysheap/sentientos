use common::{pid::Tid, syscalls::sys_wait};

extern crate userspace;

fn main() {
    match sys_wait(Tid(1)) {
        Err(common::errors::SysWaitError::NotAChild) => println!("NotAChild"),
        other => println!("Unexpected: {:?}", other),
    }
}
