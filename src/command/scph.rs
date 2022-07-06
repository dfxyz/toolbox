use crate::command::sshh;

pub const NAME: &str = "scph";

#[inline]
pub fn args<'a>() -> clap::Command<'a> {
    clap::Command::new(NAME)
        .about("Helper for scp.")
        .trailing_var_arg(true)
        .arg(clap::arg!(-h --help "Print help information."))
        .arg(clap::arg!(-e --edit "Edit the remote hosts with '$EDITOR'."))
        .arg(clap::arg!(-r --recursive "Recursively copy entire directories."))
        .arg(clap::arg!([PATH]... "Local path(s) or remote path(s)."))
}

#[inline]
pub fn run() -> ! {
    let matches = args().get_matches();
    run_with_matches(&matches);
}

#[inline]
pub fn run_with_matches(matches: &clap::ArgMatches) -> ! {
    if matches.contains_id("edit") {
        sshh::edit_sshh_entries();
    }
    match matches.get_many::<String>("PATH") {
        None => sshh::print_entries(),
        Some(paths) => {
            let entries = sshh::load_entries();
            let mut args = vec![];
            if matches.contains_id("recursive") {
                args.push("-r".to_string());
            }
            for path in paths {
                let parts = path.splitn(2, ':').collect::<Vec<&str>>();
                if parts.len() < 2 {
                    args.push(parts[0].to_string());
                } else {
                    let key = parts[0];
                    let path = parts[1];
                    match sshh::get_entry(&entries, key) {
                        None => {
                            eprintln!(
                                "error: cannot find the remote host representing by '{}'",
                                key
                            );
                            std::process::exit(1);
                        }
                        Some(e) => {
                            let destination = format!("scp://{}/{}", e.target, path);
                            args.push(destination);
                        }
                    }
                }
            }
            let args = args.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
            crate::exec("scp", &args, &[]);
        }
    };
}
