#![cfg(windows)]

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON, LoadImageW, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, WM_USER};
use std::os::windows::ffi::OsStrExt;

pub const TRAY_ID: u32 = 1;

pub struct TrayIcon { data: NOTIFYICONDATAW, added: bool, owned_icon: Option<HICON> }

impl TrayIcon {
    pub fn new(hwnd: HWND) -> Self {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_USER + 1,
            ..Default::default()
        };
        let tip: Vec<u16> = "League Ready Hotkeys\0".encode_utf16().collect();
        data.szTip[..tip.len()].copy_from_slice(&tip);
        Self { data, added: false, owned_icon: None }
    }

    pub fn with_icon(hwnd: HWND, path: &std::path::Path) -> windows::core::Result<Self> {
        let mut tray = Self::new(hwnd);
        let path: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let handle = unsafe { LoadImageW(None, windows::core::PCWSTR(path.as_ptr()), IMAGE_ICON, 0, 0, LR_LOADFROMFILE | LR_DEFAULTSIZE)? };
        let icon = HICON(handle.0);
        tray.data.hIcon = icon;
        tray.owned_icon = Some(icon);
        Ok(tray)
    }

    pub fn add(&mut self) -> bool {
        if self.added { return true; }
        self.added = unsafe { Shell_NotifyIconW(NIM_ADD, &self.data) }.as_bool();
        self.added
    }

    pub fn remove(&mut self) {
        if self.added { unsafe { let _ = Shell_NotifyIconW(NIM_DELETE, &self.data); } self.added = false; }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.remove();
        if let Some(icon) = self.owned_icon.take() { unsafe { let _ = DestroyIcon(icon); } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tray_identity_is_stable() { assert_eq!(TRAY_ID, 1); }
}
