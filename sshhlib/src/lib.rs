use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use simple_logger::{custom, error, warn};

pub struct Record {
    pub alias: String,
    pub username: String,
    pub host: String,
    pub port: u16,
}

#[cfg(windows)]
fn home_dir() -> Result<PathBuf, ()> {
    std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .map_err(|_| {
            error!("failed to get env var 'USERPROFILE'");
        })
}

#[cfg(unix)]
fn home_dir() -> Result<PathBuf, ()> {
    std::env::var("HOME").map(PathBuf::from).map_err(|_| {
        error!("failed to get env var 'HOME'");
    })
}

struct Entry {
    host: String,
    hostname: String,
    user: String,
    port: u16,
}

fn parse_entries(content: &str) -> Result<Vec<Entry>, ()> {
    let mut result = Vec::new();
    macro_rules! check_and_push {
        ($entry:ident) => {
            if $entry.hostname.is_empty() {
                warn!(
                    "ssh config contains host '{}' which has no hostname",
                    $entry.host
                );
            } else if $entry.user.is_empty() {
                warn!(
                    "ssh config contains host '{}' which has no user",
                    $entry.host
                );
            } else {
                result.push($entry);
            }
        };
    }

    enum State {
        Initial,
        Collecting(Entry),
    }
    let mut state = State::Initial;
    for (i, line) in content.lines().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.len() < 2 {
            continue; // invalid line
        }
        let word0 = words[0].to_lowercase();
        match state {
            State::Initial => {
                if word0 != "host" {
                    continue;
                }
                let entry = Entry {
                    host: words[1].to_string(),
                    hostname: String::new(),
                    user: String::new(),
                    port: 22,
                };
                state = State::Collecting(entry);
            }
            State::Collecting(ref mut entry) => match &word0 {
                x if x == "host" => {
                    let mut entry1 = Entry {
                        host: words[1].to_string(),
                        hostname: String::new(),
                        user: String::new(),
                        port: 22,
                    };
                    std::mem::swap(entry, &mut entry1);
                    check_and_push!(entry1);
                }
                x if x == "hostname" => entry.hostname = words[1].to_string(),
                x if x == "user" => entry.user = words[1].to_string(),
                x if x == "port" => {
                    entry.port = words[1].parse().map_err(|_| {
                        error!("ssh config contains invalid port option at line {}", i);
                    })?
                }
                _ => continue,
            },
        }
    }
    if let State::Collecting(entry) = state {
        check_and_push!(entry);
    }

    Ok(result)
}

fn open_and_load_config(create: bool, write: bool) -> Result<Option<(File, Vec<Entry>)>, ()> {
    let parent_dir = home_dir()?.join(".ssh");
    if create {
        std::fs::create_dir_all(&parent_dir).map_err(|e| {
            error!("failed to create ssh directory: {}", e);
        })?;
    }
    let path = parent_dir.join("config");
    let mut file = match File::options()
        .create(create)
        .write(write)
        .read(true)
        .open(path)
    {
        Ok(x) => x,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(e) => {
            error!("failed to open ssh config file: {}", e);
            return Err(());
        }
    };
    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| {
        error!("failed to read ssh config file: {}", e);
    })?;
    let entries = parse_entries(&content)?;
    Ok(Some((file, entries)))
}

pub fn list_entries() -> Result<(), ()> {
    let result = open_and_load_config(false, false)?;
    if result.is_none() {
        warn!("no entries");
        return Ok(());
    }
    let (_, entries) = result.unwrap();
    for entry in entries {
        custom!(title=entry.host; "{}@{}:{}", entry.user, entry.hostname, entry.port);
    }
    Ok(())
}

macro_rules! writeln {
    ($($tt:tt)+) => {
        ::std::writeln!($($tt)+).map_err(|e| {
            error!("failed to write ssh config file: {}", e);
        })?;
    };
}

macro_rules! write_entries {
    ($file:ident, $entries:ident) => {
        $file.seek(SeekFrom::Start(0)).map_err(|e| {
            error!("failed to seek to the start of ssh config file: {}", e);
        })?;
        $file.set_len(0).map_err(|e| {
            error!("failed to truncate ssh config file: {}", e);
        })?;
        for entry in $entries {
            writeln!($file, "Host {}", entry.host);
            writeln!($file, "    HostName {}", entry.hostname);
            writeln!($file, "    User {}", entry.user);
            writeln!($file, "    Port {}", entry.port);
        }
    };
}

fn parse_uri(uri: &str) -> Result<(&str, &str, u16), ()> {
    let mut iter = uri.splitn(2, '@');
    let user = iter.next().ok_or_else(|| {
        error!("invalid uri '{}', no username part", uri);
    })?;

    let remaining = iter.next().ok_or_else(|| {
        error!("invalid uri '{}', no host and port part", uri);
    })?;
    let mut iter = remaining.rsplitn(2, ':');
    let hostname = iter.next().ok_or_else(|| {
        error!("invalid uri '{}', no hostname part", uri);
    })?;
    if hostname.contains(':') {
        if !hostname.starts_with('[') || !hostname.ends_with(']') {
            error!(
                "invalid uri '{}', hostname contains ':' but is not a valid ipv6 address",
                uri
            );
            return Err(());
        }
    }

    let port = iter
        .next()
        .map(|x| {
            x.parse().map_err(|_| {
                error!("invalid uri '{}', port part is not a valid number", uri);
            })
        })
        .unwrap_or(Ok(22))?;

    Ok((user, hostname, port))
}

pub fn add_or_modify_entry(host: &str, uri: &str) -> Result<(), ()> {
    let (user, hostname, port) = parse_uri(uri)?;
    let (mut file, mut entries) = open_and_load_config(true, true)?.ok_or_else(|| {
        error!("unexpected: ssh config file not opened for writing");
    })?;
    let mut modified = false;
    for entry in &mut entries {
        if entry.host == host {
            modified = true;
            entry.hostname = hostname.to_string();
            entry.user = user.to_string();
            entry.port = port;
        }
    }
    if !modified {
        entries.push(Entry {
            host: host.to_string(),
            hostname: hostname.to_string(),
            user: user.to_string(),
            port,
        });
        entries.sort_by(|a, b| a.host.cmp(&b.host));
    }
    write_entries!(file, entries);
    Ok(())
}

pub fn rename_entry(host: &str, new_name: &str) -> Result<(), ()> {
    let result = open_and_load_config(false, true)?;
    if result.is_none() {
        warn!("no entries");
        return Ok(());
    }
    let (mut file, mut entries) = result.unwrap();
    let mut found = false;
    for entry in &mut entries {
        if entry.host == host {
            found = true;
            entry.host = new_name.to_string();
            continue;
        }
        if entry.host == new_name {
            error!("entry '{}' already exists", new_name);
            return Err(());
        }
    }
    if !found {
        error!("entry '{}' not found", host);
        return Err(());
    }
    entries.sort_by(|a, b| a.host.cmp(&b.host));
    write_entries!(file, entries);
    Ok(())
}

pub fn remove_entry(host: &str) -> Result<(), ()> {
    let result = open_and_load_config(false, true)?;
    if result.is_none() {
        error!("no entries to remove");
        return Ok(());
    }
    let (mut file, mut entries) = result.unwrap();
    let mut found = false;
    entries.retain(|entry| {
        if entry.host == host {
            found = true;
            false
        } else {
            true
        }
    });
    if !found {
        error!("entry '{}' not found", host);
        return Err(());
    }
    write_entries!(file, entries);
    Ok(())
}
