pub mod app;
pub mod reconnect;
pub mod lcu;
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
