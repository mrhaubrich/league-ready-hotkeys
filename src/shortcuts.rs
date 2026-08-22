use crate::app::HotkeyAction;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutKey {
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutBinding {
    pub modifiers: Vec<String>,
    pub input: String,
}

pub struct ShortcutBindings {
    pub accept: ShortcutBinding,
    pub decline: ShortcutBinding,
}

impl ShortcutBindings {
    pub fn action_for_keyboard(
        &self,
        virtual_key: u32,
        modifiers: &[&str],
    ) -> Option<HotkeyAction> {
        if self.accept.matches_keyboard(virtual_key, modifiers) {
            Some(HotkeyAction::Accept)
        } else if self.decline.matches_keyboard(virtual_key, modifiers) {
            Some(HotkeyAction::Decline)
        } else {
            None
        }
    }
    pub fn action_for_mouse(&self, button: &str) -> Option<HotkeyAction> {
        if self.accept.matches_mouse(button) {
            Some(HotkeyAction::Accept)
        } else if self.decline.matches_mouse(button) {
            Some(HotkeyAction::Decline)
        } else {
            None
        }
    }
}

impl ShortcutBinding {
    pub fn parse(value: &str) -> Result<Self, ShortcutError> {
        let parts: Vec<_> = value
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        if parts.is_empty() {
            return Err(ShortcutError::Unsupported(value.to_owned()));
        }
        let input = parts.last().unwrap().to_ascii_uppercase();
        let mut modifiers = Vec::new();
        for modifier in &parts[..parts.len() - 1] {
            let normalized = modifier.to_ascii_lowercase();
            if !matches!(normalized.as_str(), "ctrl" | "alt" | "shift" | "win") {
                return Err(ShortcutError::Unsupported(value.to_owned()));
            }
            if modifiers.contains(&normalized) {
                return Err(ShortcutError::Duplicate);
            }
            modifiers.push(normalized);
        }
        let valid_input = input.starts_with('F') && input[1..].parse::<u8>().is_ok()
            || input.starts_with("MOUSE")
            || input.len() == 1 && input.chars().next().unwrap().is_ascii_alphanumeric();
        if !valid_input {
            return Err(ShortcutError::Unsupported(value.to_owned()));
        }
        Ok(Self { modifiers, input })
    }

    pub fn matches_keyboard(&self, virtual_key: u32, modifiers: &[&str]) -> bool {
        if self.input.len() != 1 {
            return false;
        }
        let expected = self.input.as_bytes()[0] as u32;
        expected == virtual_key
            && self.modifiers.iter().all(|modifier| {
                modifiers
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(modifier))
            })
    }

    pub fn matches_mouse(&self, button: &str) -> bool {
        self.input.eq_ignore_ascii_case(button)
    }
}

impl ShortcutKey {
    pub const fn virtual_key(self) -> u32 {
        match self {
            Self::F1 => 0x70,
            Self::F2 => 0x71,
            Self::F3 => 0x72,
            Self::F4 => 0x73,
            Self::F5 => 0x74,
            Self::F6 => 0x75,
            Self::F7 => 0x76,
            Self::F8 => 0x77,
            Self::F9 => 0x78,
            Self::F10 => 0x79,
            Self::F11 => 0x7A,
            Self::F12 => 0x7B,
        }
    }
    pub fn parse(value: &str) -> Result<Self, ShortcutError> {
        match value.trim().to_ascii_uppercase().as_str() {
            "F1" => Ok(Self::F1),
            "F2" => Ok(Self::F2),
            "F3" => Ok(Self::F3),
            "F4" => Ok(Self::F4),
            "F5" => Ok(Self::F5),
            "F6" => Ok(Self::F6),
            "F7" => Ok(Self::F7),
            "F8" => Ok(Self::F8),
            "F9" => Ok(Self::F9),
            "F10" => Ok(Self::F10),
            "F11" => Ok(Self::F11),
            "F12" => Ok(Self::F12),
            _ => Err(ShortcutError::Unsupported(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutConfig {
    pub accept: ShortcutKey,
    pub decline: ShortcutKey,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            accept: ShortcutKey::F1,
            decline: ShortcutKey::F2,
        }
    }
}

impl ShortcutConfig {
    pub fn parse(accept: &str, decline: &str) -> Result<Self, ShortcutError> {
        let config = Self {
            accept: ShortcutKey::parse(accept)?,
            decline: ShortcutKey::parse(decline)?,
        };
        if config.accept == config.decline {
            return Err(ShortcutError::Duplicate);
        }
        Ok(config)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ShortcutError {
    #[error("unsupported shortcut: {0}")]
    Unsupported(String),
    #[error("accept and decline shortcuts must differ")]
    Duplicate,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_f1_f2() {
        assert_eq!(
            ShortcutConfig::default(),
            ShortcutConfig::parse("F1", "F2").unwrap()
        );
    }
    #[test]
    fn rejects_duplicates() {
        assert_eq!(
            ShortcutConfig::parse("F3", "f3"),
            Err(ShortcutError::Duplicate)
        );
    }
    #[test]
    fn rejects_unknown_keys() {
        assert!(matches!(
            ShortcutKey::parse("A"),
            Err(ShortcutError::Unsupported(_))
        ));
    }
    #[test]
    fn parses_keyboard_combo_and_mouse_button() {
        assert_eq!(ShortcutBinding::parse("Ctrl+Shift+A").unwrap().input, "A");
        assert_eq!(ShortcutBinding::parse("Mouse4").unwrap().input, "MOUSE4");
    }
    #[test]
    fn matches_keyboard_and_mouse_events() {
        let key = ShortcutBinding::parse("Ctrl+Shift+C").unwrap();
        assert!(key.matches_keyboard('C' as u32, &["ctrl", "shift"]));
        assert!(!key.matches_keyboard('C' as u32, &["ctrl"]));
        let mouse = ShortcutBinding::parse("Mouse4").unwrap();
        assert!(mouse.matches_mouse("mouse4"));
    }
    #[test]
    fn maps_events_to_distinct_actions() {
        let bindings = ShortcutBindings {
            accept: ShortcutBinding::parse("Ctrl+C").unwrap(),
            decline: ShortcutBinding::parse("Mouse4").unwrap(),
        };
        assert_eq!(
            bindings.action_for_keyboard('C' as u32, &["ctrl"]),
            Some(HotkeyAction::Accept)
        );
        assert_eq!(
            bindings.action_for_mouse("Mouse4"),
            Some(HotkeyAction::Decline)
        );
    }
}
