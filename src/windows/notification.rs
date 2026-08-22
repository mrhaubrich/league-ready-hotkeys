#![cfg(windows)]

use std::sync::atomic::{AtomicU32, Ordering};
use windows::core::Result;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateEllipticRgn, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint,
    FillRect, FillRgn, LineTo, MoveToEx, SelectObject, SetBkMode, SetTextColor, UpdateWindow,
    DT_CENTER, DT_VCENTER, HBRUSH, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
};
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetSystemMetrics,
    RegisterClassW, SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW, HTCLIENT, HWND_TOPMOST,
    SWP_NOACTIVATE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNA, WM_LBUTTONDOWN, WM_MOUSEMOVE,
    WM_PAINT, WM_SETCURSOR, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::Win32::UI::WindowsAndMessaging::{LoadCursorW, SetCursor, IDC_ARROW, IDC_HAND};

const WIDTH: i32 = 360;
const HEIGHT: i32 = 150;
pub const ACTION_NONE: u32 = 0;
pub const ACTION_ACCEPT: u32 = 1;
pub const ACTION_DECLINE: u32 = 2;
static ACTION_REQUEST: AtomicU32 = AtomicU32::new(ACTION_NONE);

pub fn take_action() -> u32 {
    ACTION_REQUEST.swap(ACTION_NONE, Ordering::AcqRel)
}

pub struct ReadyCheckNotification {
    hwnd: HWND,
}

impl ReadyCheckNotification {
    pub fn new(_owner: HWND) -> Result<Self> {
        let class = windows::core::w!("LeagueReadyHotkeysNotification");
        unsafe {
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(notification_proc),
                hInstance: GetModuleHandleW(None)?.into(),
                lpszClassName: class,
                style: CS_HREDRAW | CS_VREDRAW,
                ..Default::default()
            });
        }
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class,
                windows::core::w!("Ready check"),
                WS_POPUP,
                0,
                0,
                WIDTH,
                HEIGHT,
                None,
                None,
                None,
                None,
            )?
        };
        unsafe {
            let region = CreateRoundRectRgn(0, 0, WIDTH + 1, HEIGHT + 1, 18, 18);
            let _ = SetWindowRgn(hwnd, region, true);
        }
        Ok(Self { hwnd })
    }

    pub fn set_active(&self, active: bool) {
        if active {
            let (x, y) = unsafe {
                (
                    GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN)
                        - WIDTH
                        - 24,
                    GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN)
                        - HEIGHT
                        - 64,
                )
            };
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                let _ = ShowWindow(self.hwnd, SW_SHOWNA);
                let _ = UpdateWindow(self.hwnd);
            }
        } else {
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
    }
}

impl Drop for ReadyCheckNotification {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn notification_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_PAINT {
        let mut paint = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut paint);
        let dark = super::startup::is_dark_mode();
        let brush = CreateSolidBrush(if dark {
            COLORREF(0x001f242b)
        } else {
            COLORREF(0x00f5f7fa)
        });
        let accent = CreateSolidBrush(COLORREF(0x002fcf8f));
        let accept = CreateSolidBrush(COLORREF(0x0078a526));
        let decline = CreateSolidBrush(COLORREF(0x00706058));
        let mut rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut rect);
        let _ = FillRect(hdc, &rect, HBRUSH(brush.0));
        let logo = CreateEllipticRgn(24, 18, 56, 50);
        let _ = FillRgn(hdc, logo, HBRUSH(accent.0));
        let _ = DeleteObject(logo);
        let pen = CreatePen(PS_SOLID, 3, COLORREF(0x00ffffff));
        let old_pen = SelectObject(hdc, pen);
        let _ = MoveToEx(hdc, 32, 34, None);
        let _ = LineTo(hdc, 38, 40);
        let _ = LineTo(hdc, 49, 27);
        let _ = SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen);
        let _ = FillRect(
            hdc,
            &RECT {
                left: 0,
                top: 0,
                right: 6,
                bottom: rect.bottom,
            },
            HBRUSH(accent.0),
        );
        let accept_region = CreateRoundRectRgn(22, 92, 168, 132, 10, 10);
        let decline_region = CreateRoundRectRgn(182, 92, 338, 132, 10, 10);
        let _ = FillRgn(hdc, accept_region, HBRUSH(accept.0));
        let _ = FillRgn(hdc, decline_region, HBRUSH(decline.0));
        let _ = DeleteObject(accept_region);
        let _ = DeleteObject(decline_region);
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(
            hdc,
            if dark {
                COLORREF(0x00ffffff)
            } else {
                COLORREF(0x001f242b)
            },
        );
        let mut title: Vec<u16> = "League Ready Hotkeys\0".encode_utf16().collect();
        let _ = DrawTextW(
            hdc,
            &mut title,
            &mut RECT {
                left: 70,
                top: 14,
                right: 340,
                bottom: 42,
            },
            DT_VCENTER,
        );
        let mut body: Vec<u16> = "Ready check detected\0".encode_utf16().collect();
        let _ = SetTextColor(
            hdc,
            if dark {
                COLORREF(0x00d5dde5)
            } else {
                COLORREF(0x0039434d)
            },
        );
        let _ = DrawTextW(
            hdc,
            &mut body,
            &mut RECT {
                left: 70,
                top: 44,
                right: 340,
                bottom: 76,
            },
            DT_VCENTER,
        );
        let mut accept_text: Vec<u16> = "F1  Accept\0".encode_utf16().collect();
        let mut decline_text: Vec<u16> = "F2  Decline\0".encode_utf16().collect();
        let _ = SetTextColor(hdc, COLORREF(0x00ffffff));
        let _ = DrawTextW(
            hdc,
            &mut accept_text,
            &mut RECT {
                left: 22,
                top: 92,
                right: 168,
                bottom: 132,
            },
            DT_CENTER | DT_VCENTER | windows::Win32::Graphics::Gdi::DT_SINGLELINE,
        );
        let _ = DrawTextW(
            hdc,
            &mut decline_text,
            &mut RECT {
                left: 182,
                top: 92,
                right: 338,
                bottom: 132,
            },
            DT_CENTER | DT_VCENTER | windows::Win32::Graphics::Gdi::DT_SINGLELINE,
        );
        let _ = DeleteObject(brush);
        let _ = DeleteObject(accent);
        let _ = DeleteObject(accept);
        let _ = DeleteObject(decline);
        let _ = EndPaint(hwnd, &paint);
        return LRESULT(0);
    }
    if message == WM_SETCURSOR && (lparam.0 & 0xffff) as u32 == HTCLIENT {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    if message == WM_MOUSEMOVE {
        let x = (lparam.0 & 0xffff) as i16 as i32;
        let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
        let on_button =
            ((22..=168).contains(&x) || (182..=338).contains(&x)) && (92..=132).contains(&y);
        let cursor_id = if on_button { IDC_HAND } else { IDC_ARROW };
        if let Ok(cursor) = LoadCursorW(None, cursor_id) {
            let _ = SetCursor(cursor);
        }
        return LRESULT(0);
    }
    if message == WM_LBUTTONDOWN {
        let x = (lparam.0 & 0xffff) as i16 as i32;
        let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
        if (22..=168).contains(&x) && (92..=132).contains(&y) {
            println!("notification button clicked: accept");
            ACTION_REQUEST.store(ACTION_ACCEPT, Ordering::Release);
        } else if (182..=338).contains(&x) && (92..=132).contains(&y) {
            println!("notification button clicked: decline");
            ACTION_REQUEST.store(ACTION_DECLINE, Ordering::Release);
        } else {
            println!("notification click outside buttons: x={x} y={y}");
        }
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}
