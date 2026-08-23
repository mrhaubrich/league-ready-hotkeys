#![cfg(windows)]

use crate::shortcuts::ShortcutBinding;
use std::ffi::c_void;
use windows::core::Result;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND, DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, InvalidateRect,
    SetBkMode, SetTextColor, DT_CENTER, DT_SINGLELINE, DT_VCENTER, HBRUSH, PAINTSTRUCT,
    TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, ReleaseCapture, SetFocus};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowLongPtrW, IsZoomed,
    KillTimer, PostMessageW, RegisterClassW, SendMessageW, SetForegroundWindow, SetTimer,
    SetWindowLongPtrW, ShowWindow, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HTCAPTION, SW_HIDE,
    SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, WM_CLOSE, WM_LBUTTONDOWN, WM_NCHITTEST,
    WM_PAINT, WM_TIMER, WNDCLASSW, WS_POPUP, WS_THICKFRAME,
};

pub const SETTINGS_UPDATED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 3;

const ACCEPT_ZONE: RECT = RECT {
    left: 24,
    top: 76,
    right: 276,
    bottom: 122,
};
const DECLINE_ZONE: RECT = RECT {
    left: 24,
    top: 136,
    right: 276,
    bottom: 182,
};
const SAVE_ZONE: RECT = RECT {
    left: 190,
    top: 210,
    right: 276,
    bottom: 246,
};
const CANCEL_ZONE: RECT = RECT {
    left: 24,
    top: 210,
    right: 170,
    bottom: 246,
};
const TITLEBAR_HEIGHT: i32 = 34;
const CLOSE_ZONE: RECT = RECT {
    left: 340,
    top: 0,
    right: 380,
    bottom: 34,
};
const MAX_ZONE: RECT = RECT {
    left: 300,
    top: 0,
    right: 340,
    bottom: 34,
};
const MIN_ZONE: RECT = RECT {
    left: 260,
    top: 0,
    right: 300,
    bottom: 34,
};

pub struct HotkeySettings {
    hwnd: HWND,
    state: Box<SettingsState>,
}

struct SettingsState {
    owner: HWND,
    accept: ShortcutBinding,
    decline: ShortcutBinding,
    capture_target: u32,
    previous: [bool; 256],
    message: String,
    dirty: bool,
}

impl HotkeySettings {
    pub fn new(owner: HWND) -> Result<Self> {
        let class = windows::core::w!("LeagueReadyHotkeysSettings");
        unsafe {
            let _ = RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(settings_proc),
                hInstance: GetModuleHandleW(None)?.into(),
                lpszClassName: class,
                style: CS_HREDRAW | CS_VREDRAW,
                ..Default::default()
            });
        }
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                class,
                windows::core::w!("Configure hotkeys"),
                WS_POPUP | WS_THICKFRAME,
                0,
                0,
                380,
                314,
                None,
                None,
                None,
                None,
            )?
        };
        let (accept, decline) = crate::windows::startup::load_bindings();
        let state = Box::new(SettingsState {
            owner,
            accept,
            decline,
            capture_target: 0,
            previous: [false; 256],
            message: "Choose an action, then press a key or mouse button".to_owned(),
            dirty: false,
        });
        unsafe {
            let dark_mode = windows::Win32::Foundation::BOOL(1);
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&dark_mode as *const windows::Win32::Foundation::BOOL).cast::<c_void>(),
                std::mem::size_of_val(&dark_mode) as u32,
            );
            let corners = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                (&corners as *const DWM_WINDOW_CORNER_PREFERENCE).cast::<c_void>(),
                std::mem::size_of_val(&corners) as u32,
            );
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&raw const *state) as isize);
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
        Ok(Self { hwnd, state })
    }

    pub fn show(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = SetForegroundWindow(self.hwnd);
            let _ = SetFocus(self.hwnd);
        }
    }
}

impl Drop for HotkeySettings {
    fn drop(&mut self) {
        unsafe {
            let _ = KillTimer(self.hwnd, 1);
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
        }
        let _ = &self.state;
    }
}

fn in_zone(x: i32, y: i32, zone: RECT) -> bool {
    (zone.left..=zone.right).contains(&x) && (zone.top..=zone.bottom).contains(&y)
}

fn binding_label(binding: &ShortcutBinding) -> String {
    binding.canonical().replace("MOUSE", "MB")
}

fn modifier_down(vk: i32) -> bool {
    unsafe { GetAsyncKeyState(vk) < 0 }
}

fn capture_candidates() -> impl Iterator<Item = u32> {
    let keyboard = (0x30..=0x39).chain(0x41..=0x5A).chain(0x70..=0x7B);
    keyboard.chain([1, 2, 4, 5])
}

fn candidate_binding(vk: u32) -> Option<String> {
    let modifiers = [(0x11, "Ctrl"), (0x12, "Alt"), (0x10, "Shift")]
        .into_iter()
        .filter(|(key, _)| modifier_down(*key))
        .map(|(_, name)| name)
        .chain(if modifier_down(0x5B) || modifier_down(0x5C) {
            Some("Win")
        } else {
            None
        })
        .collect::<Vec<_>>();
    let input = match vk {
        1 => "Mouse1".to_owned(),
        2 => "Mouse2".to_owned(),
        4 => "Mouse4".to_owned(),
        5 => "Mouse5".to_owned(),
        0x70..=0x7B => format!("F{}", vk - 0x6F),
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(vk)?.to_string(),
        _ => return None,
    };
    Some(if modifiers.is_empty() {
        input
    } else {
        format!("{}+{input}", modifiers.join("+"))
    })
}

unsafe fn poll_capture(hwnd: HWND, state: &mut SettingsState) {
    for vk in capture_candidates() {
        let down = GetAsyncKeyState(vk as i32) < 0;
        let was_down = state.previous[vk as usize];
        state.previous[vk as usize] = down;
        if !down || was_down {
            continue;
        }
        let Some(value) = candidate_binding(vk) else {
            continue;
        };
        let Ok(binding) = ShortcutBinding::parse(&value) else {
            continue;
        };
        if (state.capture_target == 1 && binding == state.decline)
            || (state.capture_target == 2 && binding == state.accept)
        {
            state.message = "Those shortcuts must be different".to_owned();
            state.capture_target = 0;
            let _ = KillTimer(hwnd, 1);
            let _ = InvalidateRect(hwnd, None, false);
            return;
        }
        if state.capture_target == 1 {
            state.accept = binding;
        } else {
            state.decline = binding;
        }
        state.dirty = true;
        state.message = "Unsaved changes. Click Save to apply them.".to_owned();
        state.capture_target = 0;
        let _ = KillTimer(hwnd, 1);
        let _ = InvalidateRect(hwnd, None, false);
        return;
    }
}

unsafe extern "system" fn settings_proc(
    hwnd: HWND,
    message: u32,
    _wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsState;
    if message == WM_PAINT {
        let mut paint = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut paint);
        let background = CreateSolidBrush(COLORREF(0x00170f08));
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let _ = FillRect(hdc, &client, HBRUSH(background.0));
        let _ = DeleteObject(background);
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, COLORREF(0x00f0e6d2));
        let title_brush = CreateSolidBrush(COLORREF(0x001d2733));
        let title_rect = RECT {
            left: 0,
            top: 0,
            right: client.right,
            bottom: TITLEBAR_HEIGHT,
        };
        let _ = FillRect(hdc, &title_rect, HBRUSH(title_brush.0));
        let _ = DeleteObject(title_brush);
        let mut caption = "Configure hotkeys\0".encode_utf16().collect::<Vec<_>>();
        let _ = DrawTextW(
            hdc,
            &mut caption,
            &mut RECT {
                left: 14,
                top: 0,
                right: 250,
                bottom: TITLEBAR_HEIGHT,
            },
            DT_SINGLELINE | DT_VCENTER,
        );
        let mut close = "×\0".encode_utf16().collect::<Vec<_>>();
        let mut max = "□\0".encode_utf16().collect::<Vec<_>>();
        let mut min = "—\0".encode_utf16().collect::<Vec<_>>();
        let _ = DrawTextW(
            hdc,
            &mut min,
            &mut RECT {
                left: 260,
                top: 0,
                right: 300,
                bottom: TITLEBAR_HEIGHT,
            },
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
        let _ = DrawTextW(
            hdc,
            &mut max,
            &mut RECT {
                left: 300,
                top: 0,
                right: 340,
                bottom: TITLEBAR_HEIGHT,
            },
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
        let _ = SetTextColor(hdc, COLORREF(0x00f06b6b));
        let _ = DrawTextW(
            hdc,
            &mut close,
            &mut RECT {
                left: 340,
                top: 0,
                right: 380,
                bottom: TITLEBAR_HEIGHT,
            },
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
        if let Some(state) = state_ptr.as_ref() {
            let mut title = "CONFIGURE HOTKEYS\0".encode_utf16().collect::<Vec<_>>();
            let mut instruction = format!("{}\0", state.message)
                .encode_utf16()
                .collect::<Vec<_>>();
            let mut accept = format!("Accept:  {}\0", binding_label(&state.accept))
                .encode_utf16()
                .collect::<Vec<_>>();
            let mut decline = format!("Decline: {}\0", binding_label(&state.decline))
                .encode_utf16()
                .collect::<Vec<_>>();
            let _ = DrawTextW(
                hdc,
                &mut title,
                &mut RECT {
                    left: 24,
                    top: 52,
                    right: 300,
                    bottom: 78,
                },
                DT_SINGLELINE | DT_VCENTER,
            );
            let _ = DrawTextW(
                hdc,
                &mut instruction,
                &mut RECT {
                    left: 24,
                    top: 82,
                    right: 300,
                    bottom: 102,
                },
                DT_SINGLELINE | DT_VCENTER,
            );
            let _ = DrawTextW(
                hdc,
                &mut accept,
                &mut RECT {
                    left: 34,
                    top: 122,
                    right: 266,
                    bottom: 144,
                },
                DT_SINGLELINE | DT_VCENTER,
            );
            let _ = DrawTextW(
                hdc,
                &mut decline,
                &mut RECT {
                    left: 34,
                    top: 182,
                    right: 266,
                    bottom: 204,
                },
                DT_SINGLELINE | DT_VCENTER,
            );
            let _ = SetTextColor(hdc, COLORREF(0x00c89b3c));
            let mut accept_action = "Press to configure Accept\0"
                .encode_utf16()
                .collect::<Vec<_>>();
            let mut decline_action = "Press to configure Decline\0"
                .encode_utf16()
                .collect::<Vec<_>>();
            let _ = DrawTextW(
                hdc,
                &mut accept_action,
                &mut RECT {
                    left: 34,
                    top: 142,
                    right: 266,
                    bottom: 156,
                },
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
            let _ = DrawTextW(
                hdc,
                &mut decline_action,
                &mut RECT {
                    left: 34,
                    top: 202,
                    right: 266,
                    bottom: 216,
                },
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
            let mut cancel = "CANCEL\0".encode_utf16().collect::<Vec<_>>();
            let mut save = "SAVE\0".encode_utf16().collect::<Vec<_>>();
            let _ = SetTextColor(hdc, COLORREF(0x00ffffff));
            let mut cancel_rect = CANCEL_ZONE;
            cancel_rect.top += TITLEBAR_HEIGHT;
            cancel_rect.bottom += TITLEBAR_HEIGHT;
            let mut save_rect = SAVE_ZONE;
            save_rect.top += TITLEBAR_HEIGHT;
            save_rect.bottom += TITLEBAR_HEIGHT;
            let _ = DrawTextW(
                hdc,
                &mut cancel,
                &mut cancel_rect,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
            let _ = DrawTextW(
                hdc,
                &mut save,
                &mut save_rect,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
        }
        let _ = EndPaint(hwnd, &paint);
        return LRESULT(0);
    }
    if message == WM_LBUTTONDOWN {
        let x = (lparam.0 & 0xffff) as i16 as i32;
        let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
        if let Some(state) = state_ptr.as_mut() {
            if in_zone(x, y, CLOSE_ZONE) {
                let _ = SendMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
                return LRESULT(0);
            }
            if in_zone(x, y, MAX_ZONE) {
                let _ = ShowWindow(
                    hwnd,
                    if IsZoomed(hwnd).as_bool() {
                        SW_RESTORE
                    } else {
                        SW_MAXIMIZE
                    },
                );
                return LRESULT(0);
            }
            if in_zone(x, y, MIN_ZONE) {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
                return LRESULT(0);
            }
            if y < TITLEBAR_HEIGHT {
                let _ = ReleaseCapture();
                let _ = SendMessageW(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN,
                    WPARAM(HTCAPTION as usize),
                    LPARAM(0),
                );
                return LRESULT(0);
            }
            let y = y - TITLEBAR_HEIGHT;
            if in_zone(x, y, SAVE_ZONE) {
                if state.dirty {
                    if crate::windows::startup::save_bindings(
                        &state.accept.canonical(),
                        &state.decline.canonical(),
                    )
                    .is_ok()
                    {
                        state.dirty = false;
                        state.message = "Saved hotkeys.".to_owned();
                        let _ = PostMessageW(state.owner, SETTINGS_UPDATED, WPARAM(0), LPARAM(0));
                    } else {
                        state.message = "Could not save shortcut settings".to_owned();
                    }
                    let _ = InvalidateRect(hwnd, None, false);
                }
                return LRESULT(0);
            }
            if in_zone(x, y, CANCEL_ZONE) {
                let (accept, decline) = crate::windows::startup::load_bindings();
                state.accept = accept;
                state.decline = decline;
                state.dirty = false;
                state.capture_target = 0;
                let _ = KillTimer(hwnd, 1);
                state.message = "Changes cancelled.".to_owned();
                let _ = InvalidateRect(hwnd, None, false);
                return LRESULT(0);
            }
            if in_zone(x, y, ACCEPT_ZONE) || in_zone(x, y, DECLINE_ZONE) {
                state.capture_target = if in_zone(x, y, ACCEPT_ZONE) { 1 } else { 2 };
                state.message = "Listening… press a keyboard key or mouse button".to_owned();
                for vk in capture_candidates() {
                    state.previous[vk as usize] = GetAsyncKeyState(vk as i32) < 0;
                }
                let _ = SetTimer(hwnd, 1, 30, None);
                let _ = SetFocus(hwnd);
                let _ = InvalidateRect(hwnd, None, false);
            }
        }
        return LRESULT(0);
    }
    if message == WM_TIMER {
        if let Some(state) = state_ptr.as_mut() {
            if state.capture_target != 0 {
                poll_capture(hwnd, state);
            }
        }
        return LRESULT(0);
    }
    if message == WM_NCHITTEST {
        return LRESULT(windows::Win32::UI::WindowsAndMessaging::HTCLIENT as isize);
    }
    if message == WM_CLOSE {
        let _ = KillTimer(hwnd, 1);
        let _ = ShowWindow(hwnd, SW_HIDE);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, WPARAM(0), lparam)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_formats_keyboard_and_mouse_bindings() {
        assert_eq!(candidate_binding(0x70).as_deref(), Some("F1"));
        assert_eq!(candidate_binding(0x41).as_deref(), Some("A"));
        assert_eq!(candidate_binding(4).as_deref(), Some("Mouse4"));
    }

    #[test]
    fn displayed_binding_uses_mouse_keycap_label() {
        let binding = ShortcutBinding::parse("Mouse5").unwrap();
        assert_eq!(binding_label(&binding), "MB5");
    }
}
