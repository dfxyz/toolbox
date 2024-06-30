use std::path::PathBuf;
use std::process::ExitCode;

use simple_logger::{error, warn};

const PREDEFINED_ENV: &[(&str, &str)] = &[
    ("MSYSTEM", "UCRT64"),
    ("MSYS", "winsymlinks:nativestrict"),
    ("MSYS2_PATH_TYPE", "inherit"),
    ("CHERE_INVOKING", "1"),
];

fn main() -> ExitCode {
    main0().unwrap_or(ExitCode::FAILURE)
}

fn main0() -> Result<ExitCode, ()> {
    let shell = resolve_shell()?;

    let mut cmd = std::process::Command::new(&shell);
    cmd.args(std::env::args().skip(1));
    cmd.envs(PREDEFINED_ENV.iter().map(|(k, v)| (*k, *v)));

    unsafe { winapi::um::consoleapi::SetConsoleCtrlHandler(None, 1) };
    let mut child = cmd.spawn().map_err(|e| {
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
}

fn resolve_shell() -> Result<PathBuf, ()> {
    const SHELLS: &[&str] = &["zsh.exe", "bash.exe", "powershell.exe", "cmd.exe"];
    let path = std::env::var_os("PATH").ok_or_else(|| {
        warn!("failed to get PATH environment variable");
    })?;
    let dirs = std::env::split_paths(&path).collect::<Vec<_>>();
    for shell in SHELLS {
        for dir in &dirs {
            let shell_path = dir.join(shell);
            if shell_path.is_file() {
                return Ok(shell_path);
            }
        }
    }
    warn!("failed to find any available shell");
    Err(())
}
