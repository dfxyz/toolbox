use clap::Parser;
use std::process::ExitCode;

#[derive(Parser)]
struct Arguments {
    #[clap(help = "Path(s) to file or directory")]
    paths: Vec<String>,

    #[clap(long = "system", help = "Unset the 'SYSTEM' attribute too")]
    system: bool,
}
fn main() -> ExitCode {
    let result = || -> Result<(), ()> {
        let args = Arguments::parse();
        for path in &args.paths {
            unsafe { win_file_attr::unset(path, true, args.system) }?;
        }
        Ok(())
    };
    match result() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
