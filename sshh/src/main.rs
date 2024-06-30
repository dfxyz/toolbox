use std::process::ExitCode;

use clap::Parser;
use simple_logger::{error, warn};

// sshh # list all endpoints
// sshh -a <alias> <endpoint> # upsert an endpoint
// sshh <alias> # connect to an endpoint
// sshh <alias> <command> [args...] # run a command on an endpoint
#[derive(Parser)]
struct Arguments {
    #[clap(short, long, help = "Upsert a new endpoint")]
    add: bool,

    #[clap(help = "Alias of the endpoint")]
    alias: Option<String>,

    #[clap(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "The rest of the arguments",
        long_help = r#"The rest of the arguments.
If '--add' is present, this is required as the endpoint.
Otherwise, these are optional command and arguments to run on the endpoint."#
    )]
    rest_args: Vec<String>,
}

fn main() -> ExitCode {
    let result = || -> Result<ExitCode, ()> {
        let args = Arguments::parse();

        if args.add {
            if args.alias.is_none() || args.rest_args.is_empty() {
                error!("'--add' requires an alias and an endpoint");
                return Err(());
            }
            let alias = args.alias.as_deref().unwrap();
            let endpoint = args.rest_args[0].as_str();
            let helper = sshhlib::SshHelper::open()?;
            helper.upsert_endpoint(alias, endpoint)?;
            return Ok(ExitCode::SUCCESS);
        }

        if args.alias.is_none() {
            let helper = sshhlib::SshHelper::open()?;
            helper.list_endpoints()?;
            return Ok(ExitCode::SUCCESS);
        }

        let helper = sshhlib::SshHelper::open()?;
        let alias = args.alias.as_deref().unwrap();
        let uri = helper.get_endpoint_uri("ssh://", alias)?;

        unsafe { winapi::um::consoleapi::SetConsoleCtrlHandler(None, 1) };
        #[cfg(windows)]
        let program = "ssh.exe";
        #[cfg(unix)]
        let program = "ssh";
        let mut child = std::process::Command::new(program)
            .args(std::iter::once(uri).chain(args.rest_args.into_iter()))
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
