#![cfg(windows)]

use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN,
    WM_MBUTTONDOWN, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_XBUTTONDOWN,
};

static mut KEY_HOOK: HHOOK = HHOOK(std::ptr::null_mut());
static mut MOUSE_HOOK: HHOOK = HHOOK(std::ptr::null_mut());

pub fn run_diagnostic() {
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
        println!("keyboard input: vk={}", data.vkCode);
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
            WM_XBUTTONDOWN => "x",
            WM_MOUSEMOVE => "move",
            _ => "other",
        };
        if event != "move" {
            println!(
                "mouse input: button={} x={} y={}",
                event, data.pt.x, data.pt.y
            );
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}
