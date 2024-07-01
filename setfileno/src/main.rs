use std::process::ExitCode;

use clap::Parser;
use simple_logger::{custom_cyan, error, info, warn};

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
        let padding = args.padding;
        let filename_by_number: Box<dyn Fn(u32) -> String> = match filename.rfind('.') {
            Some(index) => {
                fileno = filename[..index].parse::<u32>().ok();
                let ext_name = filename[index..].to_string();
                Box::new(move |number: u32| -> String {
                    format!("{:0width$}{}", number, ext_name, width = padding as usize)
                })
            }
            None => {
                fileno = filename.parse::<u32>().ok();
                Box::new(|number: u32| -> String {
                    format!("{:0width$}", number, width = padding as usize)
                })
            }
        };
        let mut number = args.number;
        if filename == filename_by_number(number) {
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

        let swap;
        let reverse;
        match fileno {
            Some(n) => {
                if n < number {
                    swap = n + 1 == number && filename_by_number(n) == filename;
                    reverse = true;
                } else {
                    swap = (n == number || n - 1 == number) && filename_by_number(n) == filename;
                    reverse = false;
                }
            }
            None => {
                swap = false;
                reverse = false;
            }
        }

        let mut rename_pairs = vec![(reserved_name, filename_by_number(number))];
        if swap {
            rename_pairs.push((filename_by_number(number), filename));
        } else {
            loop {
                let last_file = &rename_pairs.last().unwrap().1;
                let md = match std::fs::metadata(last_file) {
                    Ok(md) => md,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
                    Err(e) => {
                        error!("failed to get metadata of '{}': {}", last_file, e);
                        custom_cyan!(notice: "target file has been renamed with '__' prefix");
                        return Err(());
                    }
                };
                if !md.is_file() {
                    error!("'{}' is not a file", last_file);
                    custom_cyan!(notice: "target file has been renamed with '__' prefix");
                    return Err(());
                }
                if reverse {
                    if number == 0 {
                        error!(
                            "'{}' exists and cannot be renamed as negative number",
                            last_file
                        );
                        custom_cyan!(notice: "target file has been renamed with '__' prefix");
                        return Err(());
                    }
                } else {
                    if number == u32::MAX {
                        error!("'{}' exists and cannot be renamed as zero", last_file);
                        custom_cyan!(notice: "target file has been renamed with '__' prefix");
                        return Err(());
                    }
                }
                let next_number = if reverse { number - 1 } else { number + 1 };
                rename_pairs.push((last_file.clone(), filename_by_number(next_number)));
                number = next_number;
            }
        }

        for (old_name, new_name) in rename_pairs.iter().rev() {
            std::fs::rename(old_name, new_name).map_err(|e| {
                error!("failed to rename '{}' to '{}': {}", old_name, new_name, e);
                custom_cyan!(notice: "target file has been renamed with '__' prefix");
            })?;
        }

        info!("done");
        Ok(())
    }();
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
