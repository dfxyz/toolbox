use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::PathBuf;

pub const NAME: &str = "sshh";
const ENTRIES_FILE: &str = ".sshh_entries";

#[inline]
pub fn args<'a>() -> clap::Command<'a> {
    clap::Command::new(NAME)
        .about("Helper for ssh.")
        .trailing_var_arg(true)
        .arg(clap::arg!(-h --help "Print help information."))
        .arg(clap::arg!(-e --edit "Edit the remote hosts with '$EDITOR'."))
        .arg(clap::arg!([TARGET] "An index number or alias representing the remote host to connect."))
        .arg(clap::arg!([COMMAND]... "The command (and its arguments) to execute on the remote host."))
}

#[inline]
pub fn run() -> ! {
    let matches = args().get_matches();
    run_with_matches(&matches);
}

#[inline]
pub fn run_with_matches(matches: &clap::ArgMatches) -> ! {
    if matches.contains_id("edit") {
        edit_sshh_entries();
    }
    match matches.get_one::<String>("TARGET") {
        None => print_entries(),
        Some(key) => {
            let entries = load_entries();
            let entry = match get_entry(&entries, key) {
                None => {
                    eprintln!(
                        "error: cannot find the remote host representing by '{}'",
                        key
                    );
                    std::process::exit(1);
                }
                Some(e) => e,
            };
            let destination = format!("ssh://{}", entry.target);
            let mut args = vec![destination.as_str()];
            match matches.get_many::<String>("COMMAND") {
                None => {}
                Some(commands) => {
                    commands.for_each(|s| args.push(s.as_str()));
                }
            }
            crate::exec("ssh", &args, &[]);
        }
    };
}

#[inline]
#[cfg(windows)]
fn home() -> Option<String> {
    std::env::var("USERPROFILE").ok()
}

#[inline]
#[cfg(not(windows))]
fn home() -> Option<String> {
    std::env::var("HOME").ok()
}

fn touch_entries_file() -> PathBuf {
    let home = match home() {
        None => {
            eprintln!("error: cannot determine the home directory");
            std::process::exit(1);
        }
        Some(s) => PathBuf::from(s),
    };
    let file = home.join(ENTRIES_FILE);
    match file.metadata() {
        Ok(md) => {
            if !md.is_file() {
                eprintln!(
                    "error: '{}' exists and is not a file",
                    file.to_string_lossy()
                );
                std::process::exit(1);
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            match std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&file)
            {
                Ok(_) => {}
                Err(e) => {
                    eprintln!(
                        "error: failed to create file '{}': {}",
                        file.to_string_lossy(),
                        e
                    );
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!(
                "error: cannot access the metadata of '{}': {}",
                file.to_string_lossy(),
                e
            );
            std::process::exit(1);
        }
    }
    file
}

pub(super) fn edit_sshh_entries() -> ! {
    let editor = match std::env::var("EDITOR") {
        Ok(s) => s,
        Err(_) => {
            eprintln!("error: cannot determine '$EDITOR'");
            std::process::exit(1);
        }
    };
    let file = touch_entries_file();
    crate::exec(&editor, &[file.to_str().unwrap()], &[]);
}

pub(super) struct Entry {
    pub(super) target: String,
    aliases: Vec<String>,
    comment: String,
}

pub(super) fn load_entries() -> Vec<Entry> {
    let file_path = touch_entries_file();
    let mut entries = vec![];
    let mut file_content = String::new();
    let mut file = match File::open(&file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "error: failed to open file '{}': {}",
                file_path.to_string_lossy(),
                e
            );
            std::process::exit(1);
        }
    };
    match file.read_to_string(&mut file_content) {
        Ok(_) => {}
        Err(e) => {
            eprintln!(
                "error: failed to read file '{}': {}",
                file_path.to_string_lossy(),
                e
            );
        }
    }
    for line in file_content.split('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut iter = line.splitn(2, '#');
        let s = iter.next().unwrap();
        let comment = match iter.next() {
            None => String::new(),
            Some(s) => s.trim().to_string(),
        };
        let mut iter = s.split(' ').filter(|s| !s.is_empty());
        let target = match iter.next() {
            None => continue,
            Some(s) => s.to_string(),
        };
        let aliases = iter.map(|s| s.to_string()).collect::<Vec<String>>();
        let entry = Entry {
            target,
            aliases,
            comment,
        };
        entries.push(entry);
    }
    entries
}

pub(super) fn print_entries() -> ! {
    let entries = load_entries();
    for (i, entry) in entries.iter().enumerate() {
        let mut line = format!("[{}] {}", i, entry.target);
        if !entry.aliases.is_empty() {
            let aliases = format!(" {:?}", entry.aliases);
            line.push_str(&aliases);
        }
        if !entry.comment.is_empty() {
            let comment = format!(" # {}", entry.comment);
            line.push_str(&comment);
        }
        println!("{}", line);
    }
    std::process::exit(0);
}

pub(super) fn get_entry<'a>(entries: &'a [Entry], key: &'a str) -> Option<&'a Entry> {
    let index = key.parse::<usize>().ok();
    for (i, entry) in entries.iter().enumerate() {
        match index {
            None => {}
            Some(index) => {
                if i == index {
                    return Some(entry);
                }
            }
        }
        for alias in &entry.aliases {
            if alias == key {
                return Some(entry);
            }
        }
    }
    None
}
