pub const NAME: &str = "sudo";

#[inline]
pub fn args<'a>() -> clap::Command<'a> {
    clap::Command::new(NAME)
        .about("Run commands with elevated privileges.")
        .trailing_var_arg(true)
        .allow_hyphen_values(true)
        .arg(clap::arg!(-h --help "Print help information."))
        .arg(clap::arg!(<ARG>... "Command and argument(s) to run."))
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
        .unwrap()
        .map(|s| s.as_str())
        .collect::<Vec<&str>>();
    run0(&args);
}

fn run0(args: &[&str]) -> ! {
    // <COMMAND> -> '<COMMAND>'
    // surround with single-quotes, in case of any spaces in <COMMAND>
    let cmd = args[0].replace('\'', "''"); // powershell's escape syntax
    let mut cmd_args = format!("'{}'", cmd);

    if args.len() > 1 {
        cmd_args.push(' ');

        // <ARG>... -> "<ARG>"... (normal escape) -> '"<ARG>"'... (powershell-style escape)
        // surround with double-quotes then with single-quotes,
        // to avoid powershell split arguments by space
        let args = args[1..]
            .iter()
            .map(|a| {
                let a = a.replace('"', r#"\""#).replace('\'', "''");
                format!(r#"'"{}"'"#, a)
            })
            .collect::<Vec<String>>()
            .join(",");
        cmd_args.push_str(&args);
    }

    let script = format!("& {{ Start-Process {} -Verb RunAs }}", cmd_args);
    crate::exec(
        "powershell",
        &["-WindowStyle", "Hidden", "-Command", &script],
        &[],
    );
}
