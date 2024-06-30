use std::process::ExitCode;

use clap::Parser;
use simple_logger::{error, warn};

// scph # list all endpoints
// scph -a <alias> <endpoint> # upsert an endpoint
// scph <source> ... <target> # copy files between endpoints
#[derive(Parser)]
struct Arguments {
    #[clap(short, long, help = "Upsert a new endpoint")]
    add: bool,

    #[clap(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "The rest of the arguments",
        long_help = r#"The rest of the arguments.
If '--add' is present, they are required as '<alias> <endpoint>'.
Otherwise, these are '<source> ... <target>'."#
    )]
    rest_args: Vec<String>,
}

fn main() -> ExitCode {
    let result = || -> Result<ExitCode, ()> {
        let args = Arguments::parse();

        if args.add {
            if args.rest_args.len() < 2 {
                error!("'--add' requires an alias and an endpoint");
                return Err(());
            }
            let alias = args.rest_args[0].as_str();
            let endpoint = args.rest_args[1].as_str();
            let db = sshhlib::SshHelper::open()?;
            db.upsert_endpoint(alias, endpoint)?;
            return Ok(ExitCode::SUCCESS);
        }

        let rest_args = args.rest_args;
        if rest_args.is_empty() {
            let db = sshhlib::SshHelper::open()?;
            db.list_endpoints()?;
            return Ok(ExitCode::SUCCESS);
        }

        if rest_args.len() < 2 {
            error!("'<source> ... <target>' is required");
            return Err(());
        }
        let helper = sshhlib::SshHelper::open()?;
        let mut params = Vec::with_capacity(rest_args.len());
        for arg in rest_args {
            let parts = arg.splitn(2, ':').collect::<Vec<&str>>();
            if parts.len() < 2 {
                params.push(arg);
                continue;
            }
            let alias = parts[0];
            let endpoint = helper.get_endpoint_uri("scp://", alias)?;
            let path = parts[1];
            params.push(format!("{}/{}", endpoint, path));
        }

        unsafe { winapi::um::consoleapi::SetConsoleCtrlHandler(None, 1) };
        #[cfg(windows)]
        let program = "scp.exe";
        #[cfg(unix)]
        let program = "scp";
        let mut child = std::process::Command::new(program)
            .args(params)
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
