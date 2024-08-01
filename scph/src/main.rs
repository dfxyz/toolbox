use std::process::ExitCode;

use clap::Parser;
use simple_logger::{error, warn};

#[derive(Parser)]
struct Arguments {
    #[clap(short, long, help = "Upsert a new endpoint", conflicts_with_all = &["remove", "rename"])]
    add: bool,

    #[clap(short='d', long, help = "Remove an endpoint", conflicts_with_all = &["add", "rename"])]
    remove: bool,

    #[clap(short='m', long, help = "Rename an endpoint", conflicts_with_all = &["add", "remove"])]
    rename: bool,

    #[clap(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "The rest of the arguments",
        long_help = r#"The positional arguments.
If '--add' is present, an alias and an endpoint are required.
If '--rename' is present, an alias and a new alias are required.
If '--rename' is present, an alias is required.
Otherwise, these arguments are passed to the 'scp' command."#
    )]
    args: Vec<String>,
}

fn main() -> ExitCode {
    let result = || -> Result<ExitCode, ()> {
        let args = Arguments::parse();

        if args.add {
            let args = args.args;
            if args.len() < 2 {
                error!("'--add' requires an alias and an endpoint URI");
                return Err(());
            }
            let alias = &args[0];
            let endpoint_url = &args[1];
            sshhlib::add_or_modify_entry(alias, endpoint_url)?;
            return Ok(ExitCode::SUCCESS);
        }

        if args.remove {
            let args = args.args;
            if args.len() < 1 {
                error!("'--remove' requires an alias");
                return Err(());
            }
            let alias = &args[0];
            sshhlib::remove_entry(alias)?;
            return Ok(ExitCode::SUCCESS);
        }

        if args.rename {
            let args = args.args;
            if args.len() < 2 {
                error!("'--rename' requires an alias and a new name");
                return Err(());
            }
            let alias = &args[0];
            let new_alias = &args[1];
            sshhlib::rename_entry(alias, new_alias)?;
            return Ok(ExitCode::SUCCESS);
        }

        let args = args.args;
        if args.is_empty()  {
            sshhlib::list_entries()?;
            return Ok(ExitCode::SUCCESS);
        }

        unsafe { winapi::um::consoleapi::SetConsoleCtrlHandler(None, 1) };
        #[cfg(windows)]
        let program = "scp.exe";
        #[cfg(unix)]
        let program = "scp";
        let mut child = std::process::Command::new(program)
            .args(args)
            .spawn()
            .map_err(|e| {
                error!("failed to spawn child process: {}", e);
            })?;
        let status = child.wait().map_err(|e| {
            error!("failed to wait for child process: {}", e);
        })?;
        let code = status.code().ok_or_else(|| {
            warn!("child process terminated by signal");
        })?;
        let code = u8::try_from(code).map_err(|_| {
            warn!("child process returned invalid exit code: {}", code);
        })?;
        Ok(ExitCode::from(code))
    }();
    result.unwrap_or(ExitCode::FAILURE)
}
