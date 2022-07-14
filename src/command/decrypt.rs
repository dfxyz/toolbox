use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use aes_gcm::aead::AeadInPlace;
use aes_gcm::{Aes128Gcm, Nonce};

use crate::command::encrypt;

pub const NAME: &str = "decrypt";

struct DecryptInput {
    iv: [u8; 12],
    buf: Vec<u8>,
}

#[inline]
pub fn args<'a>() -> clap::Command<'a> {
    clap::Command::new(NAME)
        .about("Decrypt file(s) with AES-128-GCM.")
        .trailing_var_arg(true)
        .arg(clap::arg!(-h --help "Print help information.").display_order(0))
        .arg(clap::arg!(-k --key <SECRET> "The secret key."))
        .arg(
            clap::arg!(-e --ext <EXT> "The extension name for encrypted file(s).")
                .default_value("enc")
                .required(false),
        )
        .arg(
            clap::arg!(-o --outdir <DIR> "The directory to output decrypted file(s).")
                .default_value(".")
                .required(false),
        )
        .arg(clap::arg!(-f --force "Overwrite existed file(s) without confirmation."))
        .arg(clap::arg!(<FILE>... "The file(s) to decrypt."))
}

#[inline]
pub fn run() -> ! {
    let matches = args().get_matches();
    run_with_matches(&matches);
}

#[inline]
pub fn run_with_matches(matches: &clap::ArgMatches) -> ! {
    let key = matches.get_one::<String>("key").unwrap();
    if key.is_empty() {
        eprintln!("error: secret key is empty");
        std::process::exit(1);
    }
    let cipher = encrypt::create_cipher(key);

    let ext = matches.get_one::<String>("ext").unwrap();
    if ext.is_empty() {
        eprintln!("error: extension name is empty");
        std::process::exit(1);
    }
    let ext_suffix = format!(".{}", ext);

    let outdir = matches.get_one::<String>("outdir").unwrap();
    let outdir = PathBuf::from(outdir);
    if !outdir.is_dir() {
        eprintln!(
            "error: '{}' is not a valid directory",
            outdir.to_string_lossy()
        );
        std::process::exit(1);
    }

    let overwrite = matches.contains_id("force");

    let cpu_num = num_cpus::get();
    let (in_txs, out_rxs) = encrypt::create_channel_and_workers(cpu_num, cipher, do_decrypt);

    'file_loop: for file in matches.get_many::<String>("FILE").unwrap() {
        let file_path = PathBuf::from(file);
        let base_name = match file_path.file_name() {
            Some(n) => n.to_string_lossy(),
            None => {
                eprintln!(
                    "error: cannot determine file name of '{}'",
                    file_path.to_string_lossy()
                );
                continue 'file_loop;
            }
        };
        let base_name = match base_name.strip_suffix(&ext_suffix) {
            None => {
                println!(
                    "skip file '{}'; not ends with '{}'",
                    file_path.to_string_lossy(),
                    ext_suffix
                );
                continue 'file_loop;
            }
            Some(s) => s,
        };
        let mut file = match File::open(&file_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!(
                    "error: failed to open file '{}': {}",
                    file_path.to_string_lossy(),
                    e
                );
                continue 'file_loop;
            }
        };
        let outfile_path = outdir.join(base_name);
        if outfile_path.is_file() && !overwrite {
            print!("overwrite '{}'? [y/N]\n>> ", outfile_path.to_string_lossy());
            match std::io::stdout().flush() {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("error: failed to flush stdout: {}", e);
                    std::process::exit(1);
                }
            }
            let mut answer = "".to_string();
            match std::io::stdin().read_line(&mut answer) {
                Ok(_) => {
                    let answer = answer.trim();
                    if answer != "y" && answer != "Y" {
                        println!("skip file '{}'", file_path.to_string_lossy());
                        continue 'file_loop;
                    }
                }
                Err(e) => {
                    eprintln!("error: failed to read from stdin: {}", e);
                    std::process::exit(1);
                }
            }
        }
        let mut outfile = match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&outfile_path)
        {
            Ok(f) => f,
            Err(e) => {
                eprintln!(
                    "error: failed to open file '{}': {}",
                    outfile_path.to_string_lossy(),
                    e
                );
                continue 'file_loop;
            }
        };

        println!("decrypting file '{}'...", file_path.to_string_lossy());
        loop {
            let mut done = false;
            let mut i = 0;
            while i < cpu_num {
                let mut head = [0u8; 16];
                match file.read(&mut head) {
                    Ok(len) => {
                        if len == 0 {
                            done = true;
                            break;
                        }
                        if len < 16 {
                            eprintln!(
                                "error: insufficient bytes read from '{}'",
                                file_path.to_string_lossy()
                            );
                            continue 'file_loop;
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "error: failed to read '{}': {}",
                            file_path.to_string_lossy(),
                            e
                        );
                        continue 'file_loop;
                    }
                }
                let mut len = [0u8; 4];
                len.clone_from_slice(&head[..4]);
                let len = u32::from_le_bytes(len);
                let mut iv = [0u8; 12];
                iv.clone_from_slice(&head[4..]);
                let mut buf = vec![0u8; len as usize];
                match file.read(&mut buf) {
                    Ok(len) => {
                        if len < buf.len() {
                            eprintln!(
                                "error: insufficient bytes read from '{}'",
                                file_path.to_string_lossy()
                            );
                            continue 'file_loop;
                        }
                        in_txs[i].send(DecryptInput { iv, buf }).unwrap();
                        i += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "error: failed to read '{}': {}",
                            file_path.to_string_lossy(),
                            e
                        );
                        continue 'file_loop;
                    }
                }
            }
            for rx in &out_rxs[..i] {
                match rx.recv().unwrap() {
                    Ok(buf) => match outfile.write_all(&buf) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!(
                                "error: failed to write '{}': {}",
                                outfile_path.to_string_lossy(),
                                e
                            );
                            continue 'file_loop;
                        }
                    },
                    Err(_) => {
                        eprintln!("error: failed to decrypt '{}'", file_path.to_string_lossy());
                        continue 'file_loop;
                    }
                }
            }
            if done {
                break;
            }
        }
    }

    std::process::exit(0);
}

fn do_decrypt(
    cipher: Arc<Aes128Gcm>,
    in_rx: crossbeam_channel::Receiver<DecryptInput>,
    out_tx: crossbeam_channel::Sender<Result<Vec<u8>, ()>>,
) {
    while let Ok(input) = in_rx.recv() {
        let iv = input.iv;
        let iv = Nonce::from(iv);
        let mut buf = input.buf;
        let msg = match cipher.decrypt_in_place(&iv, b"", &mut buf) {
            Ok(_) => Ok(buf),
            Err(_) => Err(()),
        };
        match out_tx.send(msg) {
            Ok(_) => {}
            Err(_) => return,
        }
    }
}
