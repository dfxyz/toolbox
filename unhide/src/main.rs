use clap::Parser;
use std::process::ExitCode;

#[derive(Parser)]
struct Arguments {
    #[clap(help = "Path to a file or directory")]
    path: String,

    #[clap(long = "system", help = "Unset the 'SYSTEM' attribute too")]
    system: bool,
}
fn main() -> ExitCode {
    let result = || -> Result<(), ()> {
        let args = Arguments::parse();
        unsafe { win_file_attr::unset(&args.path, true, args.system) }
    };
    match result() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
