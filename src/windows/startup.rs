#![cfg(windows)]

pub const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

use crate::shortcuts::ShortcutConfig;
use std::path::Path;
use windows::core::{w, Error, Result};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW, HKEY_CURRENT_USER,
    KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_DWORD,
    RRF_RT_REG_SZ,
};

fn key() -> Result<windows::Win32::System::Registry::HKEY> {
    let mut handle = windows::Win32::System::Registry::HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            None,
            &mut handle,
            None,
        )
    };
    if status.0 != 0 {
        return Err(Error::from_win32());
    }
    Ok(handle)
}

pub fn set_enabled(executable: &Path, enabled: bool) -> Result<()> {
    let handle = key()?;
    let result = if enabled {
        let value = format!("\"{}\"", executable.display());
        let mut bytes = value
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        bytes.extend_from_slice(&[0, 0]);
        unsafe { RegSetValueExW(handle, w!("LeagueReadyHotkeys"), 0, REG_SZ, Some(&bytes)) }
    } else {
        unsafe { RegDeleteValueW(handle, w!("LeagueReadyHotkeys")) }
    };
    unsafe {
        let _ = RegCloseKey(handle);
    }
    if result.0 != 0 && !(result.0 == 2 && !enabled) {
        return Err(Error::from_win32());
    }
    Ok(())
}

pub fn is_enabled() -> Result<bool> {
    let handle = key()?;
    let mut size = 0u32;
    let status = unsafe {
        RegGetValueW(
            handle,
            None,
            w!("LeagueReadyHotkeys"),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
    };
    unsafe {
        let _ = RegCloseKey(handle);
    }
    Ok(status.0 == 0)
}

pub fn is_dark_mode() -> bool {
    let mut handle = windows::Win32::System::Registry::HKEY::default();
    let opened = unsafe {
        windows::Win32::System::Registry::RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            0,
            KEY_QUERY_VALUE,
            &mut handle,
        )
    };
    if opened.0 != 0 {
        return false;
    }
    let mut value = 1u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            handle,
            None,
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        )
    };
    unsafe {
        let _ = RegCloseKey(handle);
    }
    status.0 == 0 && value == 0
}

pub fn notifications_enabled() -> Result<bool> {
    let mut handle = windows::Win32::System::Registry::HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\LeagueReadyHotkeys"),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            None,
            &mut handle,
            None,
        )
    };
    if status.0 != 0 {
        return Err(Error::from_win32());
    }
    let mut value = 1u32;
    let mut size = 4u32;
    let read = unsafe {
        RegGetValueW(
            handle,
            None,
            w!("NotificationsEnabled"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        )
    };
    unsafe {
        let _ = RegCloseKey(handle);
    }
    Ok(read.0 != 0 || value != 0)
}

pub fn set_notifications_enabled(enabled: bool) -> Result<()> {
    let mut handle = windows::Win32::System::Registry::HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\LeagueReadyHotkeys"),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            None,
            &mut handle,
            None,
        )
    };
    if status.0 != 0 {
        return Err(Error::from_win32());
    }
    let value = if enabled { 1u32 } else { 0u32 };
    let result = unsafe {
        RegSetValueExW(
            handle,
            w!("NotificationsEnabled"),
            0,
            windows::Win32::System::Registry::REG_DWORD,
            Some(&value.to_ne_bytes()),
        )
    };
    unsafe {
        let _ = RegCloseKey(handle);
    }
    if result.0 != 0 {
        Err(Error::from_win32())
    } else {
        Ok(())
    }
}

pub fn load_shortcuts() -> ShortcutConfig {
    let mut handle = windows::Win32::System::Registry::HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\LeagueReadyHotkeys"),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            None,
            &mut handle,
            None,
        )
    };
    if status.0 != 0 {
        return ShortcutConfig::default();
    }
    let read = |name| -> Option<String> {
        let mut buffer = [0u16; 16];
        let mut size = (buffer.len() * 2) as u32;
        let status = unsafe {
            RegGetValueW(
                handle,
                None,
                name,
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if status.0 != 0 {
            return None;
        }
        Some(
            String::from_utf16_lossy(&buffer[..size as usize / 2])
                .trim_end_matches('\0')
                .to_owned(),
        )
    };
    let accept = read(w!("AcceptShortcut"));
    let decline = read(w!("DeclineShortcut"));
    unsafe {
        let _ = RegCloseKey(handle);
    }
    match (accept, decline) {
        (Some(a), Some(d)) => ShortcutConfig::parse(&a, &d).unwrap_or_default(),
        _ => ShortcutConfig::default(),
    }
}

pub fn save_shortcuts(config: ShortcutConfig) -> Result<()> {
    let mut handle = windows::Win32::System::Registry::HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\LeagueReadyHotkeys"),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            None,
            &mut handle,
            None,
        )
    };
    if status.0 != 0 {
        return Err(Error::from_win32());
    }
    let names = [
        (w!("AcceptShortcut"), format!("{:?}", config.accept)),
        (w!("DeclineShortcut"), format!("{:?}", config.decline)),
    ];
    for (name, value) in names {
        let bytes = value
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .chain([0, 0])
            .collect::<Vec<_>>();
        let status = unsafe { RegSetValueExW(handle, name, 0, REG_SZ, Some(&bytes)) };
        if status.0 != 0 {
            unsafe {
                let _ = RegCloseKey(handle);
            }
            return Err(Error::from_win32());
        }
    }
    unsafe {
        let _ = RegCloseKey(handle);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn startup_key_is_stable() {
        assert!(super::RUN_KEY.ends_with("\\Run"));
    }
}
