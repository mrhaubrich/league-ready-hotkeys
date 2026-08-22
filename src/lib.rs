pub mod lcu;
pub mod app;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState { WaitingForClient, Connecting, Idle, ReadyCheck, Recovering }
