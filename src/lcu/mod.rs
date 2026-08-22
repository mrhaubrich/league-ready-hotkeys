mod discovery;
#[cfg(windows)]
pub use discovery::discover_lockfile;
pub use discovery::{parse_lockfile, LcuCredentials, LockfileError};
mod ready_check;
pub use ready_check::{parse_ready_check, parse_ready_check_event, ReadyCheck, ReadyCheckError};
#[cfg(windows)]
pub mod transport;

pub const READY_CHECK: &str = "/lol-matchmaking/v1/ready-check";
pub const ACCEPT: &str = "/lol-matchmaking/v1/ready-check/accept";
pub const DECLINE: &str = "/lol-matchmaking/v1/ready-check/decline";
