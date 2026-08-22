use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Clone, PartialEq, Eq)]
pub struct LcuCredentials {
    pub port: u16,
    pub password: String,
    pub protocol: String,
}

impl fmt::Debug for LcuCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LcuCredentials")
            .field("port", &self.port)
            .field("password", &"[REDACTED]")
            .field("protocol", &self.protocol)
            .finish()
    }
}

impl LcuCredentials {
    pub fn base_url(&self) -> String {
        format!("{}://127.0.0.1:{}", self.protocol, self.port)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LockfileError {
    #[error("lockfile has the wrong number of fields")]
    FieldCount,
    #[error("lockfile port is invalid")]
    InvalidPort,
    #[error("lockfile protocol is invalid")]
    InvalidProtocol,
    #[cfg(windows)]
    #[error("LeagueClientUx.exe is not running")]
    ClientNotRunning,
    #[cfg(windows)]
    #[error("League executable path is unavailable")]
    ExecutableUnavailable,
    #[cfg(windows)]
    #[error("League lockfile is unavailable")]
    LockfileUnavailable,
}

#[cfg(windows)]
pub fn discover_lockfile() -> Result<PathBuf, LockfileError> {
    use sysinfo::System;
    let mut system = System::new();
    system.refresh_processes();
    let process = system
        .processes_by_name("LeagueClientUx.exe")
        .next()
        .ok_or(LockfileError::ClientNotRunning)?;
    let executable = process.exe().ok_or(LockfileError::ExecutableUnavailable)?;
    let lockfile = executable
        .parent()
        .ok_or(LockfileError::ExecutableUnavailable)?
        .join("lockfile");
    if !lockfile.is_file() {
        return Err(LockfileError::LockfileUnavailable);
    }
    Ok(lockfile)
}

pub fn parse_lockfile(input: &str) -> Result<LcuCredentials, LockfileError> {
    let fields: Vec<_> = input.trim().split(':').collect();
    if fields.len() != 5 {
        return Err(LockfileError::FieldCount);
    }
    let port = fields[2].parse().map_err(|_| LockfileError::InvalidPort)?;
    let protocol = fields[4].to_owned();
    if protocol != "https" {
        return Err(LockfileError::InvalidProtocol);
    }
    Ok(LcuCredentials {
        port,
        password: fields[3].to_owned(),
        protocol,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_lockfile() {
        let c = parse_lockfile("LeagueClientUx:1234:54321:secret:https").unwrap();
        assert_eq!(c.port, 54321);
        assert_eq!(c.password, "secret");
    }
    #[test]
    fn rejects_bad_shape() {
        assert_eq!(parse_lockfile("bad"), Err(LockfileError::FieldCount));
    }
    #[test]
    fn rejects_non_https() {
        assert_eq!(
            parse_lockfile("x:1:2:p:http"),
            Err(LockfileError::InvalidProtocol)
        );
    }

    #[test]
    fn debug_output_redacts_password() {
        let credentials = parse_lockfile("x:1:2:super-secret:https").unwrap();
        let debug = format!("{credentials:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("REDACTED"));
    }
}
