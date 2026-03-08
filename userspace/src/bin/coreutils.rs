use std::ffi::OsString;

fn main() {
    let args: Vec<OsString> = std::env::args_os().collect();
    let binary = args[0]
        .to_str()
        .and_then(|s| s.rsplit('/').next())
        .unwrap_or("coreutils");

    let (util, util_args) = if binary == "coreutils" {
        let name = args.get(1).and_then(|s| s.to_str()).unwrap_or_else(|| {
            eprintln!("usage: coreutils UTILITY [ARGS...]");
            std::process::exit(1);
        });
        let mut new_args = vec![OsString::from(name)];
        new_args.extend_from_slice(&args[2..]);
        (name.to_owned(), new_args)
    } else {
        (binary.to_owned(), args)
    };

    let exit_code = match util.as_str() {
        "cat" => uu_cat::uumain(util_args.into_iter()),
        "echo" => uu_echo::uumain(util_args.into_iter()),
        "false" => uu_false::uumain(util_args.into_iter()),
        "ls" => uu_ls::uumain(util_args.into_iter()),
        "mkdir" => uu_mkdir::uumain(util_args.into_iter()),
        "pwd" => uu_pwd::uumain(util_args.into_iter()),
        "rm" => uu_rm::uumain(util_args.into_iter()),
        "touch" => uu_touch::uumain(util_args.into_iter()),
        "true" => uu_true::uumain(util_args.into_iter()),
        _ => {
            eprintln!("coreutils: unknown utility '{util}'");
            1
        }
    };

    std::process::exit(exit_code);
}
