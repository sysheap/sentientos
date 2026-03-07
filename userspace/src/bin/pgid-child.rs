extern crate userspace;

unsafe extern "C" {
    fn getpgid(pid: i32) -> i32;
    fn getsid(pid: i32) -> i32;
    fn setpgid(pid: i32, pgid: i32) -> i32;
    fn setsid() -> i32;
    fn getpid() -> i32;
    fn getppid() -> i32;
}

fn main() {
    let pid = unsafe { getpid() };
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "setpgid" {
        let ret = unsafe { setpgid(0, 0) };
        assert_eq!(ret, 0, "setpgid(0,0) should succeed");
        let new_pgid = unsafe { getpgid(0) };
        assert_eq!(new_pgid, pid, "after setpgid(0,0), pgid should equal pid");
        println!("child_pgid={new_pgid}");
    } else if args.len() > 1 && args[1] == "setsid" {
        // Ensure we're not a PG leader (dash may have set pgid=pid).
        // Join parent's group first so setsid can succeed.
        let ppid = unsafe { getppid() };
        let _ = unsafe { setpgid(0, ppid) };
        let ret = unsafe { setsid() };
        assert_eq!(ret, pid, "setsid should return new sid == pid");
        let new_sid = unsafe { getsid(0) };
        assert_eq!(new_sid, pid, "after setsid, sid should equal pid");
        let new_pgid = unsafe { getpgid(0) };
        assert_eq!(new_pgid, pid, "after setsid, pgid should equal pid");
        println!("child_sid={new_sid}");
    }

    println!("OK");
}
