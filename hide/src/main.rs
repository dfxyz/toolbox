use std::process::ExitCode;

use clap::Parser;

#[derive(Parser)]
struct Arguments {
    #[clap(help = "Path to a file or directory")]
    path: String,

    #[clap(long = "system", help = "Set the 'SYSTEM' attribute too")]
    system: bool,
}
fn main() -> ExitCode {
    let result = || -> Result<(), ()> {
        let args = Arguments::parse();
        unsafe { win_file_attr::set(&args.path, true, args.system) }
    };
    match result() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
