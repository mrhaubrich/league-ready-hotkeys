#![cfg(windows)]

use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos, LoadImageW,
    SetForegroundWindow, TrackPopupMenu, HICON, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE,
    MF_STRING, TPM_RIGHTBUTTON, WM_USER,
};

pub const TRAY_ID: u32 = 1;
pub const TRAY_MESSAGE: u32 = WM_USER + 1;
pub const MENU_EXIT: u32 = 1;

pub struct TrayIcon {
    data: NOTIFYICONDATAW,
    added: bool,
    owned_icon: Option<HICON>,
}

impl TrayIcon {
    pub fn new(hwnd: HWND) -> Self {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: TRAY_MESSAGE,
            ..Default::default()
        };
        let tip: Vec<u16> = "League Ready Hotkeys\0".encode_utf16().collect();
        data.szTip[..tip.len()].copy_from_slice(&tip);
        Self {
            data,
            added: false,
            owned_icon: None,
        }
    }

    pub fn with_icon(hwnd: HWND, path: &std::path::Path) -> windows::core::Result<Self> {
        let mut tray = Self::new(hwnd);
        let path: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe {
            LoadImageW(
                None,
                windows::core::PCWSTR(path.as_ptr()),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            )?
        };
        let icon = HICON(handle.0);
        tray.data.hIcon = icon;
        tray.owned_icon = Some(icon);
        Ok(tray)
    }

    pub fn add(&mut self) -> bool {
        if self.added {
            return true;
        }
        // Clean up a stale shell entry left by an interrupted diagnostic/process exit.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.data);
        }
        self.added = unsafe { Shell_NotifyIconW(NIM_ADD, &self.data) }.as_bool();
        self.added
    }

    pub fn remove(&mut self) {
        if self.added {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.data);
            }
            self.added = false;
        }
    }

    pub fn show_menu(&self) -> bool {
        let menu = unsafe { CreatePopupMenu().expect("create tray menu") };
        unsafe {
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_EXIT as usize,
                windows::core::w!("Exit"),
            );
        }
        let mut point = windows::Win32::Foundation::POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut point);
            let _ = SetForegroundWindow(self.data.hWnd);
        }
        let _ = unsafe {
            TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                self.data.hWnd,
                None,
            )
        };
        unsafe {
            let _ = DestroyMenu(menu);
        }
        true
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.remove();
        if let Some(icon) = self.owned_icon.take() {
            unsafe {
                let _ = DestroyIcon(icon);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tray_identity_is_stable() {
        assert_eq!(TRAY_ID, 1);
    }
}
