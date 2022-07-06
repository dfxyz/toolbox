use std::process::Command;

mod command;

pub fn main() {
    let program = std::env::args().next().unwrap();
    let program = program.strip_suffix(".exe").unwrap_or(&program);
    let program = program
        .rfind('/')
        .map(|i| &program[i + 1..])
        .unwrap_or(program);
    let program = program
        .rfind('\\')
        .map(|i| &program[i + 1..])
        .unwrap_or(program);

    if program == "toolbox" {
        run();
    }
    match program {
        #[cfg(windows)]
        command::sudo::NAME => command::sudo::run(),
        #[cfg(windows)]
        command::shell::NAME => command::shell::run(),
        #[cfg(windows)]
        command::hide::NAME => command::hide::run(),
        #[cfg(windows)]
        command::unhide::NAME => command::unhide::run(),
        command::sshh::NAME => command::sshh::run(),
        command::scph::NAME => command::scph::run(),
        #[cfg(windows)]
        command::wt::NAME => command::wt::run(),
        _ => {
            eprintln!("error: cannot run as '{}'", program);
            std::process::exit(1);
        }
    }
}

fn run() -> ! {
    let args = clap::Command::new("toolbox")
        .about("A set of helper programs in one executable file.")
        .hide_possible_values(true)
        .disable_help_subcommand(true)
        .arg(clap::arg!(-h --help "Print help information.").display_order(usize::MAX))
        .arg(
            {
                let arg = clap::arg!(-l --link <SUBCOMMAND> "Make a symlink of <SUBCOMMAND> in the working directory.")
                    .required(false)
                    .possible_value(command::sshh::NAME)
                    .possible_value(command::scph::NAME);
                #[cfg(windows)]
                let arg = arg.possible_value(command::sudo::NAME)
                    .possible_value(command::shell::NAME)
                    .possible_value(command::hide::NAME)
                    .possible_value(command::unhide::NAME)
                    .possible_value(command::wt::NAME);
                arg
            }
        );
    #[cfg(windows)]
    let args = args
        .subcommand(command::sudo::args().display_order(1))
        .subcommand(command::shell::args().display_order(2))
        .subcommand(command::hide::args().display_order(3))
        .subcommand(command::unhide::args().display_order(4));
    let args = args
        .subcommand(command::sshh::args().display_order(5))
        .subcommand(command::scph::args().display_order(6));
    #[cfg(windows)]
    let args = args.subcommand(command::wt::args().display_order(7));
    let matches = args.get_matches();

    if let Some(cmd) = matches.get_one::<String>("link") {
        link_subcommand(cmd);
    }

    if let Some((cmd, matches)) = matches.subcommand() {
        match cmd {
            #[cfg(windows)]
            command::sudo::NAME => command::sudo::run_with_matches(matches),
            #[cfg(windows)]
            command::shell::NAME => command::shell::run_with_matches(matches),
            #[cfg(windows)]
            command::hide::NAME => command::hide::run_with_matches(matches),
            #[cfg(windows)]
            command::unhide::NAME => command::unhide::run_with_matches(matches),
            command::sshh::NAME => command::sshh::run_with_matches(matches),
            command::scph::NAME => command::scph::run_with_matches(matches),
            #[cfg(windows)]
            command::wt::NAME => command::wt::run_with_matches(matches),
            _ => unreachable!(),
        }
    }

    println!("Try '--help' for more information.");
    std::process::exit(0);
}

#[cfg(windows)]
fn exec(program: &str, args: &[&str], envs: &[(&str, &str)]) -> ! {
    use winapi::shared::minwindef::TRUE;
    use winapi::um::consoleapi::SetConsoleCtrlHandler;
    unsafe { SetConsoleCtrlHandler(None, TRUE) };
    let mut command = Command::new(program);
    for arg in args {
        command.arg(arg);
    }
    for (k, v) in envs {
        command.env(k, v);
    }
    match command.spawn() {
        Ok(mut child) => match child.wait() {
            Ok(es) => {
                let code = es.code().unwrap_or_default();
                std::process::exit(code);
            }
            Err(e) => {
                eprintln!(
                    "error: failed to wait for sub-process '{} {:?}': {}",
                    program, args, e
                );
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!(
                "error: failed to spawn sub-process '{} {:?}': {}",
                program, args, e
            );
            std::process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn exec(program: &str, args: &[&str], envs: &[(&str, &str)]) -> ! {
    use std::process::CommandExt;

    let mut command = Command::new(cmd);
    for arg in args {
        command.arg(arg);
    }
    for (k, v) in envs {
        command.env(k, v);
    }
    let e = command.exec();
    eprintln!("error: failed to exec '{} {:?}': {}", cmd, args, e);
    std::process::exit(1);
}

#[cfg(windows)]
fn link_subcommand(cmd: &str) -> ! {
    let program = std::env::args().next().unwrap();
    exec(
        "cmd",
        &["/c", "mklink", &format!("{}.exe", cmd), &program],
        &[],
    );
}

#[cfg(not(windows))]
fn link_subcommand(cmd: &str) -> ! {
    let program = std::env::args().next().unwrap();
    exec("ln", &["-s", &format!("$(pwd)/{}", program), cmd], &[]);
}

#[cfg(windows)]
fn utf16_str(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
