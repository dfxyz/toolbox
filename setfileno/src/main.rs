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

        let reserved_name = format!("__{}", filename);
        std::fs::rename(&filename, &reserved_name).map_err(|e| {
            error!(
                "failed to rename '{}' to '{}': {}",
                filename, reserved_name, e
            );
        })?;

        let padding = args.padding;
        let filename_by_number: Box<dyn Fn(u32) -> String> = match filename.rfind('.') {
            Some(index) => {
                let ext_name = filename[index..].to_string();
                Box::new(move |number: u32| -> String {
                    format!("{:0width$}{}", number, ext_name, width = padding as usize)
                })
            }
            None => Box::new(|number: u32| -> String {
                format!("{:0width$}", number, width = padding as usize)
            }),
        };

        let mut number = args.number;
        let mut rename_pairs = vec![(reserved_name, filename_by_number(number))];
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
            rename_pairs.push((last_file.clone(), filename_by_number(number + 1)));
            number += 1;
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
