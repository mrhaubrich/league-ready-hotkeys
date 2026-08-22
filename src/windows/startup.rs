#![cfg(windows)]

pub const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

use std::path::Path;
use windows::core::{w, Error, Result};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW, HKEY_CURRENT_USER,
    KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
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

#[cfg(test)]
mod tests {
    #[test]
    fn startup_key_is_stable() {
        assert!(super::RUN_KEY.ends_with("\\Run"));
    }
}
