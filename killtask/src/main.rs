use clap::Parser;
use simple_logger::{error, info, warn};

#[derive(Parser)]
struct Arguments {
    #[clap(help = "PID(s) or image name(s) to kill")]
    targets: Vec<String>,
}
fn main() {
    let args = Arguments::parse();
    for target in args.targets {
        match target.parse::<u32>() {
            Ok(_) => {
                let _ = taskkill("/pid", &target);
            }
            Err(_) => {
                let _ = taskkill("/im", &target);
            }
        }
    }
}

fn taskkill(option: &str, target: &str) -> Result<(), ()> {
    let mut child = std::process::Command::new("taskkill.exe")
        .arg(option)
        .arg(target)
        .spawn()
        .map_err(|e| {
            error!("failed to spawn child process: {}", e);
        })?;
    child.wait().map_err(|e| {
        error!("failed to wait for child process: {}", e);
    })?;
    Ok(())
}
