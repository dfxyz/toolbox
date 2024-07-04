use std::process::ExitCode;

use clap::Parser;
use simple_logger::{custom_cyan, error, warn};

#[derive(Parser)]
struct Arguments {
    #[clap(help = "The name of the file to be renamed in number format.")]
    filename: String,

    #[clap(help = "The number to be used in the new filename.")]
    number: u32,

    #[clap(short, long, help = "The padding of the number.", default_value = "3")]
    padding: u32,
}

fn main() -> ExitCode {
    let result = || -> Result<(), ()> {
        let args = Arguments::parse();

        let filename = args.filename;
        if filename.contains('/') || filename.contains('\\') {
            warn!("target file not in the working directory");
            return Err(());
        }
        let md = std::fs::metadata(&filename).map_err(|e| {
            error!("failed to get metadata of '{}': {}", filename, e);
        })?;
        if !md.is_file() {
            error!("target is not a file");
            return Err(());
        }

        let fileno;
        let ext_suffix;
        match filename.rfind('.') {
            None => {
                fileno = filename.parse::<u32>().ok();
                ext_suffix = None;
            }
            Some(x) => {
                fileno = filename[..x].parse::<u32>().ok();
                ext_suffix = Some(&filename[x..]);
            }
        }

        let padding = args.padding;
        let gen_filename = |number| -> String {
            let mut name = format!("{:0width$}", number, width = padding as usize);
            if let Some(ext) = ext_suffix {
                name.push_str(ext);
            }
            name
        };

        let number = args.number;
        if filename == gen_filename(number) {
            warn!("no need to set file number");
            return Ok(());
        }

        let reserved_name = format!("__{}", filename);
        std::fs::rename(&filename, &reserved_name).map_err(|e| {
            error!(
                "failed to rename '{}' to '{}': {}",
                filename, reserved_name, e
            );
        })?;
        let mut rename_pairs = vec![(reserved_name, gen_filename(number))];

        enum RenameMethod {
            Swap,
            Forward,
            Backward,
        }
        let method = match fileno {
            Some(n) => {
                if n == number {
                    RenameMethod::Forward
                } else if n < number {
                    if n + 1 == number {
                        RenameMethod::Swap
                    } else {
                        RenameMethod::Backward
                    }
                } else {
                    if n - 1 == number {
                        RenameMethod::Swap
                    } else {
                        RenameMethod::Forward
                    }
                }
            }
            None => RenameMethod::Forward,
        };

        match method {
            RenameMethod::Swap => {
                rename_pairs.push((gen_filename(number), filename));
            }
            _ => {
                let mut target_number = number;
                loop {
                    let target_filename = &rename_pairs.last().unwrap().1;
                    let md = match std::fs::metadata(target_filename) {
                        Ok(md) => md,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
                        Err(e) => {
                            error!("failed to get metadata of '{}': {}", target_filename, e);
                            custom_cyan!(notice: "given file has been renamed with '__' prefix");
                            return Err(());
                        }
                    };
                    if !md.is_file() {
                        error!("'{}' is not a file", target_filename);
                        custom_cyan!(notice: "given file has been renamed with '__' prefix");
                        return Err(());
                    }

                    let next_number = if matches!(method, RenameMethod::Forward) {
                        target_number.checked_add(1).ok_or_else(|| {
                            error!("'{}' exists and is `u32::MAX`", target_filename);
                            custom_cyan!(notice: "given file has been renamed with '__' prefix");
                        })?
                    } else {
                        target_number.checked_sub(1).ok_or_else(|| {
                            error!("'{}' exists and is zero", target_filename);
                            custom_cyan!(notice: "given file has been renamed with '__' prefix");
                        })?
                    };
                    rename_pairs.push((target_filename.clone(), gen_filename(next_number)));
                    target_number = next_number;
                }
            }
        }

        for (old_name, new_name) in rename_pairs.iter().rev() {
            std::fs::rename(old_name, new_name).map_err(|e| {
                error!("failed to rename '{}' to '{}': {}", old_name, new_name, e);
                custom_cyan!(notice: "given file has been renamed with '__' prefix");
            })?;
        }

        Ok(())
    }();
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
