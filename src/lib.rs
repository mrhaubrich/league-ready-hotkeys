pub mod app;
pub mod lcu;
pub mod reconnect;
pub mod shortcuts;
#[cfg(windows)]
pub mod windows;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    WaitingForClient,
    Connecting,
    Idle,
    ReadyCheck,
    Recovering,
}
