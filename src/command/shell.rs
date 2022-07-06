use std::path::PathBuf;

pub const NAME: &str = "shell";

const ENVS: [(&str, &str); 4] = [
    ("CHERE_INVOKING", "1"),
    ("MSYSTEM", "MINGW64"),
    ("MSYS", "winsymlinks:nativestrict"),
    ("MSYS2_PATH_TYPE", "inherit"),
];

const SHELLS: [&str; 2] = ["zsh.exe", "bash.exe"];

#[inline]
pub fn args<'a>() -> clap::Command<'a> {
    clap::Command::new(NAME)
        .about("Run a shell with custom MSYS2 environment variables.")
        .trailing_var_arg(true)
        .allow_hyphen_values(true)
        .arg(clap::arg!(-h --help "Print help information."))
        .arg(clap::arg!([ARG]... "Argument(s) to pass to the shell."))
}

#[inline]
pub fn run() -> ! {
    let matches = args().get_matches();
    run_with_matches(&matches);
}

#[inline]
pub fn run_with_matches(matches: &clap::ArgMatches) -> ! {
    let args = matches
        .get_many::<String>("ARG")
        .map(|values| values.map(|s| s.as_str()).collect::<Vec<&str>>())
        .unwrap_or_default();
    let mut shell = None;
    for sh in SHELLS {
        shell = which(sh);
        if shell.is_some() {
            break;
        }
    }
    if shell.is_none() {
        eprintln!("error: cannot find any shell");
        std::process::exit(1);
    }

    let shell = shell.unwrap();
    let shell = shell.to_str().unwrap();
    crate::exec(shell, &args, &ENVS);
}

fn which(cmd: &str) -> Option<PathBuf> {
    let path = std::env::var("PATH").unwrap();
    for dir in path.split(';') {
        let path = PathBuf::from(dir).join(cmd);
        if path.exists() {
            return Some(path);
        }
    }
    None
}
