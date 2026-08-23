#![cfg(windows)]

use crate::shortcuts::ShortcutConfig;
use windows::core::Result;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT};

pub const ACCEPT_HOTKEY_ID: i32 = 1;
pub const DECLINE_HOTKEY_ID: i32 = 2;

pub struct HotkeyManager {
    hwnd: HWND,
    enabled: bool,
}

trait Registrar {
    fn register(&mut self, id: i32, virtual_key: u32) -> Result<()>;
    fn unregister(&mut self, id: i32);
}

fn register_pair(registrar: &mut impl Registrar, config: ShortcutConfig) -> Result<()> {
    registrar.register(ACCEPT_HOTKEY_ID, config.accept.virtual_key())?;
    if let Err(error) = registrar.register(DECLINE_HOTKEY_ID, config.decline.virtual_key()) {
        registrar.unregister(ACCEPT_HOTKEY_ID);
        return Err(error);
    }
    Ok(())
}

struct WindowsRegistrar {
    hwnd: HWND,
}

impl Registrar for WindowsRegistrar {
    fn register(&mut self, id: i32, virtual_key: u32) -> Result<()> {
        unsafe { RegisterHotKey(self.hwnd, id, MOD_NOREPEAT, virtual_key) }
    }

    fn unregister(&mut self, id: i32) {
        unsafe {
            let _ = UnregisterHotKey(self.hwnd, id);
        }
    }
}

impl HotkeyManager {
    pub const fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            enabled: false,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        self.set_enabled_with_config(enabled, ShortcutConfig::default())
    }

    pub fn set_enabled_with_config(&mut self, enabled: bool, config: ShortcutConfig) -> Result<()> {
        if enabled == self.enabled {
            return Ok(());
        }
        if enabled {
            register_pair(&mut WindowsRegistrar { hwnd: self.hwnd }, config)?;
            self.enabled = true;
        } else {
            let mut registrar = WindowsRegistrar { hwnd: self.hwnd };
            registrar.unregister(ACCEPT_HOTKEY_ID);
            registrar.unregister(DECLINE_HOTKEY_ID);
            self.enabled = false;
        }
        Ok(())
    }

    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        let _ = self.set_enabled(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeRegistrar {
        calls: Vec<String>,
        fail_on: Option<i32>,
    }

    impl Registrar for FakeRegistrar {
        fn register(&mut self, id: i32, _virtual_key: u32) -> Result<()> {
            self.calls.push(format!("register:{id}"));
            if self.fail_on == Some(id) {
                Err(windows::core::Error::from_win32())
            } else {
                Ok(())
            }
        }

        fn unregister(&mut self, id: i32) {
            self.calls.push(format!("unregister:{id}"));
        }
    }

    #[test]
    fn starts_disabled_and_can_remain_disabled() {
        let mut manager = HotkeyManager::new(HWND(std::ptr::null_mut()));
        assert!(!manager.is_enabled());
        manager.set_enabled(false).expect("disabled is idempotent");
        assert!(!manager.is_enabled());
    }

    #[test]
    fn uses_distinct_command_ids() {
        assert_ne!(ACCEPT_HOTKEY_ID, DECLINE_HOTKEY_ID);
    }

    #[test]
    fn failed_second_registration_rolls_back_first() {
        let mut registrar = FakeRegistrar {
            fail_on: Some(DECLINE_HOTKEY_ID),
            ..Default::default()
        };
        assert!(register_pair(&mut registrar, ShortcutConfig::default()).is_err());
        assert_eq!(
            registrar.calls,
            ["register:1", "register:2", "unregister:1"]
        );
    }

    #[test]
    fn successful_registration_keeps_both_bindings() {
        let mut registrar = FakeRegistrar::default();
        register_pair(&mut registrar, ShortcutConfig::default()).unwrap();
        assert_eq!(registrar.calls, ["register:1", "register:2"]);
    }
}
