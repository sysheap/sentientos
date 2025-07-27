macro_rules! stub {
    ($name:ident) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name() {
            unimplemented!(concat!(
                stringify!($name),
                " newlib syscall is not implemented"
            ));
        }
    };
}

stub!(chdir);
stub!(_close);
stub!(__dso_handle);
stub!(fchmod);
stub!(fchmodat);
stub!(_fstat);
stub!(getcwd);
stub!(_getentropy);
stub!(_getpid);
stub!(_gettimeofday);
stub!(_isatty);
stub!(_kill);
stub!(_link);
stub!(_lseek);
stub!(mkdir);
stub!(_open);
stub!(pathconf);
stub!(_read);
stub!(readlink);
stub!(_sbrk);
stub!(sleep);
stub!(_stat);
stub!(symlink);
stub!(truncate);
stub!(_unlink);
stub!(usleep);
stub!(killpg);
