mod discovery;
pub use discovery::{parse_lockfile, LcuCredentials, LockfileError};
#[cfg(windows)]
pub mod transport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadyCheckResponse { None, Accepted, Declined }

pub const READY_CHECK: &str = "/lol-matchmaking/v1/ready-check";
pub const ACCEPT: &str = "/lol-matchmaking/v1/ready-check/accept";
pub const DECLINE: &str = "/lol-matchmaking/v1/ready-check/decline";
