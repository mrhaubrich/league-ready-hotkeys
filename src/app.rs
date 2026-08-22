use crate::AppState;

/// Pure application state transitions. Windows and transport code stay outside this module.
pub fn ready_check_state(active: bool) -> AppState {
    if active { AppState::ReadyCheck } else { AppState::Idle }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn active_check_enables_ready_state() { assert_eq!(ready_check_state(true), AppState::ReadyCheck); }
    #[test] fn inactive_check_disables_ready_state() { assert_eq!(ready_check_state(false), AppState::Idle); }
}
