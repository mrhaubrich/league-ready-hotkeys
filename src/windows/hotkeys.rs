#![cfg(windows)]

use windows::core::Result;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT, VK_F1, VK_F2};

pub const ACCEPT_HOTKEY_ID: i32 = 1;
pub const DECLINE_HOTKEY_ID: i32 = 2;

pub struct HotkeyManager {
    hwnd: HWND,
    enabled: bool,
}

impl HotkeyManager {
    pub const fn new(hwnd: HWND) -> Self { Self { hwnd, enabled: false } }

    pub fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        if enabled == self.enabled { return Ok(()); }
        if enabled {
            unsafe { RegisterHotKey(self.hwnd, ACCEPT_HOTKEY_ID, MOD_NOREPEAT, VK_F1.0 as u32)?; }
            if let Err(error) = unsafe { RegisterHotKey(self.hwnd, DECLINE_HOTKEY_ID, MOD_NOREPEAT, VK_F2.0 as u32) } {
                unsafe { let _ = UnregisterHotKey(self.hwnd, ACCEPT_HOTKEY_ID); }
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

    pub const fn is_enabled(&self) -> bool { self.enabled }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) { let _ = self.set_enabled(false); }
}
