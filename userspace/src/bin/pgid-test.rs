extern crate userspace;

unsafe extern "C" {
    fn getpgid(pid: i32) -> i32;
    fn getsid(pid: i32) -> i32;
    fn getpid() -> i32;
}

fn main() {
    let pid = unsafe { getpid() };

    let pgid = unsafe { getpgid(0) };
    println!("pgid={pgid}");
    assert!(pgid > 0, "pgid should be positive");

    let sid = unsafe { getsid(0) };
    println!("sid={sid}");
    assert!(sid > 0, "sid should be positive");

    // getpgid/getsid with explicit pid should match
    let pgid2 = unsafe { getpgid(pid) };
    assert_eq!(pgid2, pgid, "getpgid(pid) should match getpgid(0)");
    let sid2 = unsafe { getsid(pid) };
    assert_eq!(sid2, sid, "getsid(pid) should match getsid(0)");

    // pgid and sid should be inherited from the chain (same value)
    assert_eq!(pgid, sid, "pgid and sid should match (inherited from init)");

    println!("OK");
}
