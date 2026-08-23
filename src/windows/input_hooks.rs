#![cfg(windows)]

use crate::shortcuts::ShortcutBindings;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN,
    WM_MBUTTONDOWN, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_XBUTTONDOWN,
};

static BINDINGS: OnceLock<ShortcutBindings> = OnceLock::new();
static ACTION: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub fn take_action() -> Option<crate::app::HotkeyAction> {
    match ACTION.swap(0, std::sync::atomic::Ordering::AcqRel) {
        1 => Some(crate::app::HotkeyAction::Accept),
        2 => Some(crate::app::HotkeyAction::Decline),
        _ => None,
    }
}
pub fn install() -> bool {
    let (accept, decline) = crate::windows::startup::load_bindings();
    let _ = BINDINGS.set(ShortcutBindings { accept, decline });
    unsafe {
        KEY_HOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), HINSTANCE::default(), 0)
            .unwrap_or_default();
        MOUSE_HOOK = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), HINSTANCE::default(), 0)
            .unwrap_or_default();
        !KEY_HOOK.0.is_null() && !MOUSE_HOOK.0.is_null()
    }
}
pub fn uninstall() {
    unsafe {
        let _ = UnhookWindowsHookEx(KEY_HOOK);
        let _ = UnhookWindowsHookEx(MOUSE_HOOK);
        KEY_HOOK = HHOOK(std::ptr::null_mut());
        MOUSE_HOOK = HHOOK(std::ptr::null_mut());
    }
}

static mut KEY_HOOK: HHOOK = HHOOK(std::ptr::null_mut());
static mut MOUSE_HOOK: HHOOK = HHOOK(std::ptr::null_mut());

pub fn run_diagnostic() {
    let (accept, decline) = crate::windows::startup::load_bindings();
    let _ = BINDINGS.set(ShortcutBindings { accept, decline });
    unsafe {
        KEY_HOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), HINSTANCE::default(), 0)
            .unwrap_or_default();
        MOUSE_HOOK = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), HINSTANCE::default(), 0)
            .unwrap_or_default();
    }
    println!("input hook diagnostic active for 30 seconds");
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        let mut message = windows::Win32::UI::WindowsAndMessaging::MSG::default();
        unsafe {
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                DispatchMessageW(&message);
                if Instant::now() >= deadline {
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    unsafe {
        let _ = UnhookWindowsHookEx(KEY_HOOK);
        let _ = UnhookWindowsHookEx(MOUSE_HOOK);
        KEY_HOOK = HHOOK(std::ptr::null_mut());
        MOUSE_HOOK = HHOOK(std::ptr::null_mut());
    }
    println!("input hook diagnostic complete");
}

unsafe extern "system" fn key_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_KEYDOWN {
        let data = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let mut modifiers = Vec::new();
        if GetAsyncKeyState(0x11) < 0 {
            modifiers.push("ctrl");
        }
        if GetAsyncKeyState(0x10) < 0 {
            modifiers.push("shift");
        }
        if GetAsyncKeyState(0x12) < 0 {
            modifiers.push("alt");
        }
        if GetAsyncKeyState(0x5B) < 0 || GetAsyncKeyState(0x5C) < 0 {
            modifiers.push("win");
        }
        let action = BINDINGS
            .get()
            .and_then(|bindings| bindings.action_for_keyboard(data.vkCode, &modifiers));
        println!("keyboard input: vk={} action={action:?}", data.vkCode);
        if let Some(action) = action {
            ACTION.store(
                if action == crate::app::HotkeyAction::Accept {
                    1
                } else {
                    2
                },
                std::sync::atomic::Ordering::Release,
            );
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let event = match wparam.0 as u32 {
            WM_LBUTTONDOWN => "left",
            WM_RBUTTONDOWN => "right",
            WM_MBUTTONDOWN => "middle",
            WM_XBUTTONDOWN if (data.mouseData >> 16) == 1 => "mouse4",
            WM_XBUTTONDOWN => "mouse5",
            WM_MOUSEMOVE => "move",
            _ => "other",
        };
        if event != "move" {
            let action = BINDINGS
                .get()
                .and_then(|bindings| bindings.action_for_mouse(event));
            println!(
                "mouse input: button={} x={} y={} action={action:?}",
                event, data.pt.x, data.pt.y
            );
            if let Some(action) = action {
                ACTION.store(
                    if action == crate::app::HotkeyAction::Accept {
                        1
                    } else {
                        2
                    },
                    std::sync::atomic::Ordering::Release,
                );
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}
