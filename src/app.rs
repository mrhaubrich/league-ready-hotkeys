use crate::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction { Accept, Decline }

#[derive(Debug, Default)]
pub struct ActionGate { active: bool, in_flight: bool }

impl ActionGate {
    pub fn update_ready_check(&mut self, active: bool) {
        self.active = active;
        if !active { self.in_flight = false; }
    }

    pub fn begin(&mut self, action: HotkeyAction) -> Option<HotkeyAction> {
        if !self.active || self.in_flight { return None; }
        self.in_flight = true;
        Some(action)
    }

    pub fn finish(&mut self) { self.in_flight = false; }
}

/// Pure application state transitions. Windows and transport code stay outside this module.
pub fn ready_check_state(active: bool) -> AppState {
    if active {
        AppState::ReadyCheck
    } else {
        AppState::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_check_enables_ready_state() {
        assert_eq!(ready_check_state(true), AppState::ReadyCheck);
    }
    #[test]
    fn inactive_check_disables_ready_state() {
        assert_eq!(ready_check_state(false), AppState::Idle);
    }

    #[test]
    fn actions_require_active_check_and_suppress_duplicates() {
        let mut gate = ActionGate::default();
        assert_eq!(gate.begin(HotkeyAction::Accept), None);
        gate.update_ready_check(true);
        assert_eq!(gate.begin(HotkeyAction::Accept), Some(HotkeyAction::Accept));
        assert_eq!(gate.begin(HotkeyAction::Decline), None);
        gate.finish();
        assert_eq!(gate.begin(HotkeyAction::Decline), Some(HotkeyAction::Decline));
    }

    #[test]
    fn leaving_ready_check_clears_in_flight_action() {
        let mut gate = ActionGate::default();
        gate.update_ready_check(true);
        assert!(gate.begin(HotkeyAction::Accept).is_some());
        gate.update_ready_check(false);
        assert_eq!(gate.begin(HotkeyAction::Accept), None);
    }
}
