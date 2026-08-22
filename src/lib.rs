pub mod app;
pub mod lcu;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    WaitingForClient,
    Connecting,
    Idle,
    ReadyCheck,
    Recovering,
}
