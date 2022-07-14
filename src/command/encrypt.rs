use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use aes_gcm::aead::{AeadInPlace, NewAead};
use aes_gcm::{Aes128Gcm, Key, Nonce};
use sha2::{Digest, Sha256};

pub const NAME: &str = "encrypt";
pub(super) const BLOCK_SIZE: usize = 4096;

struct EncryptOutput {
    iv: [u8; 12],
    buf: Vec<u8>,
}

#[inline]
pub fn args<'a>() -> clap::Command<'a> {
    clap::Command::new(NAME)
        .about("Encrypt file(s) with AES-128-GCM.")
        .trailing_var_arg(true)
        .arg(clap::arg!(-h --help "Print help information.").display_order(0))
        .arg(clap::arg!(-k --key <SECRET> "The secret key."))
        .arg(
            clap::arg!(-e --ext <EXT> "The extension name for encrypted file(s).")
                .default_value("enc")
                .required(false),
        )
        .arg(
            clap::arg!(-o --outdir <DIR> "The directory to output encrypted file(s).")
                .default_value(".")
                .required(false),
        )
        .arg(clap::arg!(-f --force "Overwrite existed file(s) without confirmation."))
        .arg(clap::arg!(<FILE>... "The file(s) to encrypt."))
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
    let cipher = create_cipher(key);

    let ext = matches.get_one::<String>("ext").unwrap();
    if ext.is_empty() {
        eprintln!("error: extension name is empty");
        std::process::exit(1);
    }

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
    let (in_txs, out_rxs) = create_channel_and_workers(cpu_num, cipher, do_encrypt);

    'file_loop: for file in matches.get_many::<String>("FILE").unwrap() {
        let file_path = PathBuf::from(file);
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
        let outfile_path = outdir.join(format!("{}.{}", base_name, ext));
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

        println!("encrypting file '{}'...", file_path.to_string_lossy());
        loop {
            let mut done = false;
            let mut i = 0;
            while i < cpu_num {
                let mut buf = vec![0; BLOCK_SIZE];
                match file.read(&mut buf) {
                    Ok(len) => {
                        if len == 0 {
                            done = true;
                            break;
                        }
                        buf.truncate(len);
                        in_txs[i].send(buf).unwrap();
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
                    Ok(result) => {
                        let len = (result.buf.len() as u32).to_le_bytes();
                        match outfile.write_all(&len[..]) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!(
                                    "error: failed to write '{}': {}",
                                    outfile_path.to_string_lossy(),
                                    e
                                );
                                continue 'file_loop;
                            }
                        }
                        match outfile.write_all(&result.iv[..]) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!(
                                    "error: failed to write '{}': {}",
                                    outfile_path.to_string_lossy(),
                                    e
                                );
                                continue 'file_loop;
                            }
                        }
                        match outfile.write_all(&result.buf) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!(
                                    "error: failed to write '{}': {}",
                                    outfile_path.to_string_lossy(),
                                    e
                                );
                                continue 'file_loop;
                            }
                        }
                    }
                    Err(_) => {
                        eprintln!("error: failed to encrypt '{}'", file_path.to_string_lossy());
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

pub(super) fn create_cipher(key: &str) -> Arc<Aes128Gcm> {
    let hash = Sha256::digest(key.as_bytes());
    let mut key = [0; 16];
    key.clone_from_slice(&hash[..16]);
    let key = Key::from(key);
    Arc::new(Aes128Gcm::new(&key))
}

pub(super) fn create_channel_and_workers<CIPHER, IN, OUT>(
    num: usize,
    cipher: Arc<CIPHER>,
    worker: fn(Arc<CIPHER>, crossbeam_channel::Receiver<IN>, crossbeam_channel::Sender<OUT>),
) -> (
    Vec<crossbeam_channel::Sender<IN>>,
    Vec<crossbeam_channel::Receiver<OUT>>,
)
where
    CIPHER: Send + Sync + 'static,
    IN: Send + 'static,
    OUT: Send + 'static,
{
    assert!(num > 0);
    let mut in_txs = vec![];
    let mut out_rxs = vec![];
    for _ in 0..num {
        let (in_tx, in_rx) = crossbeam_channel::unbounded::<IN>();
        in_txs.push(in_tx);
        let (out_tx, out_rx) = crossbeam_channel::unbounded::<OUT>();
        out_rxs.push(out_rx);
        let cipher = cipher.clone();
        std::thread::spawn(move || {
            worker(cipher, in_rx, out_tx);
        });
    }
    (in_txs, out_rxs)
}

fn do_encrypt(
    cipher: Arc<Aes128Gcm>,
    in_rx: crossbeam_channel::Receiver<Vec<u8>>,
    out_tx: crossbeam_channel::Sender<Result<EncryptOutput, ()>>,
) {
    while let Ok(mut buf) = in_rx.recv() {
        let iv: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&iv);
        let msg = match cipher.encrypt_in_place(nonce, b"", &mut buf) {
            Ok(_) => Ok(EncryptOutput { iv, buf }),
            Err(_) => Err(()),
        };
        match out_tx.send(msg) {
            Ok(_) => {}
            Err(_) => return,
        }
    }
}
