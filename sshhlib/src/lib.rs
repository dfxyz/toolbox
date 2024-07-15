use std::path::PathBuf;

use simple_logger::{custom, error, warn};
use sqlite3::Connection;

const SCHEMA: &str = r#"
create table if not exists record (
    alias text not null primary key,
    username text not null,
    host text not null,
    port integer not null
);
"#;

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
fn homedir() -> Result<PathBuf, ()> {
    std::env::var("HOME").map(PathBuf::from).map_err(|_| {
        error!("failed to get env var 'HOME'");
    })
}

pub struct SshHelper {
    db: Connection,
}

impl SshHelper {
    pub fn open() -> Result<Self, ()> {
        let home = home_dir()?;
        let ssh_dir = home.join(".ssh");
        if !ssh_dir.exists() {
            std::fs::create_dir(&ssh_dir).map_err(|_| {
                error!("'.ssh' not exists and failed to create it");
            })?;
        }
        let db_path = ssh_dir.join("sshh.db");

        let conn = Connection::open(&db_path).map_err(|e| {
            error!("failed to open database file: {}", e);
        })?;
        conn.execute(SCHEMA).map_err(|e| {
            error!("failed to execute schema sql: {}", e);
        })?;
        Ok(Self { db: conn })
    }

    pub fn list_endpoints(&self) -> Result<(), ()> {
        let records = self.get_all()?;
        if records.is_empty() {
            warn!("no endpoints");
        } else {
            for record in records {
                custom!(title=record.alias; "{}@{}:{}", record.username, record.host, record.port);
            }
        }
        Ok(())
    }

    fn get_all(&self) -> Result<Vec<Record>, ()> {
        let mut stmt = self
            .db
            .prepare("select alias, username, host, port from record order by alias")
            .unwrap();
        let mut records = Vec::new();
        while let sqlite3::State::Row = stmt.next().map_err(|e| {
            error!("failed to update query state: {}", e);
        })? {
            let alias = stmt.read::<String>(0).unwrap();
            let username = stmt.read::<String>(1).unwrap();
            let host = stmt.read::<String>(2).unwrap();
            let port = stmt.read::<i64>(3).unwrap() as u16;
            records.push(Record {
                alias,
                username,
                host,
                port,
            });
        }
        Ok(records)
    }

    pub fn get_endpoint_uri(&self, prefix: &str, alias: &str) -> Result<String, ()> {
        let record = self.get(alias)?.ok_or_else(|| {
            error!("no endpoint found with alias '{}'", alias);
        })?;
        Ok(format!(
            "{}{}@{}:{}",
            prefix, record.username, record.host, record.port
        ))
    }

    fn get(&self, alias: &str) -> Result<Option<Record>, ()> {
        let mut stmt = self
            .db
            .prepare("select username, host, port from record where alias = ?")
            .unwrap();
        stmt.bind(1, alias).unwrap();
        let state = stmt.next().map_err(|e| {
            error!("failed to update query state: {}", e);
        })?;
        if state == sqlite3::State::Done {
            return Ok(None);
        }
        let alias = alias.to_string();
        let username = stmt.read::<String>(0).unwrap();
        let host = stmt.read::<String>(1).unwrap();
        let port = stmt.read::<i64>(2).unwrap() as u16;
        Ok(Some(Record {
            alias,
            username,
            host,
            port,
        }))
    }

    pub fn upsert_endpoint(&self, alias: &str, endpoint: &str) -> Result<(), ()> {
        let alias = check_alias(alias)?;
        let (username, host, port) = parse_endpoint(endpoint)?;
        let record = Record {
            alias,
            username,
            host,
            port,
        };
        self.upsert(&record)
    }

    fn upsert(&self, record: &Record) -> Result<(), ()> {
        let mut stmt = self
            .db
            .prepare(
                "insert or replace into record (alias, username, host, port) values (?, ?, ?, ?)",
            )
            .unwrap();
        stmt.bind(1, record.alias.as_str()).unwrap();
        stmt.bind(2, record.username.as_str()).unwrap();
        stmt.bind(3, record.host.as_str()).unwrap();
        stmt.bind(4, record.port as i64).unwrap();
        stmt.next().map_err(|e| {
            error!("failed to upsert the record: {}", e);
        })?;
        Ok(())
    }

    fn changes(&self) -> Result<usize, ()> {
        let mut stmt = self.db.prepare("select changes()").unwrap();
        stmt.next().map_err(|e| {
            error!("failed to get change count: {}", e);
        })?;
        let count = stmt.read::<i64>(0).unwrap();
        Ok(count as usize)
    }

    pub fn remove_endpoint(&self, alias: &str) -> Result<(), ()> {
        let mut stmt = self
            .db
            .prepare("delete from record where alias = ?")
            .unwrap();
        stmt.bind(1, alias).unwrap();
        stmt.next().map_err(|e| {
            error!("failed to remove the record: {}", e);
        })?;
        let count = self.changes()?;
        if count == 0 {
            error!("no endpoint found with alias '{}'", alias);
        }
        Ok(())
    }

    pub fn rename_endpoint(&self, old_alias: &str, new_alias: &str) -> Result<(), ()> {
        let new_alias = check_alias(new_alias)?;
        let mut stmt = self
            .db
            .prepare("update record set alias = ? where alias = ?")
            .unwrap();
        stmt.bind(1, new_alias.as_str()).unwrap();
        stmt.bind(2, old_alias).unwrap();
        stmt.next().map_err(|e| {
            error!("failed to rename the record: {}", e);
        })?;
        let count = self.changes()?;
        if count == 0 {
            error!("no endpoint found with alias '{}'", old_alias);
        }
        Ok(())
    }
}

fn check_alias(alias: &str) -> Result<String, ()> {
    let alias = alias.trim();
    if alias.is_empty() {
        error!("alias cannot be empty");
    }
    alias
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_')
        .then(|| alias.to_string())
        .ok_or_else(|| {
            error!("alias contains invalid character(s)");
        })
}

fn parse_endpoint(endpoint: &str) -> Result<(String, String, u16), ()> {
    let parts: Vec<&str> = endpoint.split('@').collect();
    if parts.len() != 2 {
        error!("invalid endpoint format");
    }
    let username = parts[0];
    let parts: Vec<&str> = parts[1].split(':').collect();
    if parts.len() > 2 {
        error!("invalid endpoint format");
    }
    let host = parts[0];
    let port = match parts.get(1) {
        None => 22u16,
        Some(x) => x.parse().map_err(|_| {
            error!("failed to parse port number");
        })?,
    };
    Ok((username.to_string(), host.to_string(), port))
}
