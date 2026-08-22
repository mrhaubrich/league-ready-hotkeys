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
            unsafe {
                RegisterHotKey(
                    self.hwnd,
                    ACCEPT_HOTKEY_ID,
                    MOD_NOREPEAT,
                    config.accept.virtual_key(),
                )?;
            }
            if let Err(error) = unsafe {
                RegisterHotKey(
                    self.hwnd,
                    DECLINE_HOTKEY_ID,
                    MOD_NOREPEAT,
                    config.decline.virtual_key(),
                )
            } {
                unsafe {
                    let _ = UnregisterHotKey(self.hwnd, ACCEPT_HOTKEY_ID);
                }
                return Err(error);
            }
            self.enabled = true;
        } else {
            unsafe {
                let _ = UnregisterHotKey(self.hwnd, ACCEPT_HOTKEY_ID);
                let _ = UnregisterHotKey(self.hwnd, DECLINE_HOTKEY_ID);
            }
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
}
