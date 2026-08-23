#![cfg(windows)]

use crate::shortcuts::ShortcutBinding;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use windows::core::Result;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateEllipticRgn, CreateFontW, CreatePen, CreateSolidBrush, DeleteObject,
    DrawTextW, EndPaint, FillRect, FillRgn, GetStockObject, InvalidateRect, LineTo, MoveToEx,
    RoundRect, SelectObject, SetBkMode, SetTextColor, UpdateWindow, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FW_NORMAL,
    FW_SEMIBOLD, HBRUSH, NULL_BRUSH, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PROOF_QUALITY, PS_SOLID,
    TRANSPARENT,
};
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetSystemMetrics,
    GetWindowLongPtrW, KillTimer, RegisterClassW, SetTimer, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HTCLIENT, HWND_TOPMOST, SWP_NOACTIVATE,
    SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNA, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_PAINT,
    WM_SETCURSOR, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::Win32::UI::WindowsAndMessaging::{LoadCursorW, SetCursor, IDC_ARROW, IDC_HAND};

const WIDTH: i32 = 420;
const HEIGHT: i32 = 212;
const READY_CHECK_DURATION_SECS: f32 = 12.0;
const ACCEPT_RECT: RECT = RECT {
    left: 20,
    top: 105,
    right: 250,
    bottom: 181,
};
const DECLINE_RECT: RECT = RECT {
    left: 262,
    top: 105,
    right: 400,
    bottom: 181,
};
pub const ACTION_NONE: u32 = 0;
pub const ACTION_ACCEPT: u32 = 1;
pub const ACTION_DECLINE: u32 = 2;
static ACTION_REQUEST: AtomicU32 = AtomicU32::new(ACTION_NONE);

pub fn take_action() -> u32 {
    ACTION_REQUEST.swap(ACTION_NONE, Ordering::AcqRel)
}

pub struct ReadyCheckNotification {
    hwnd: HWND,
    _state: Box<NotificationState>,
}

struct NotificationState {
    accept: BindingVisual,
    decline: BindingVisual,
    timer: Mutex<TimerProgress>,
}

struct TimerProgress {
    elapsed: f32,
    updated_at: Instant,
}

#[derive(Debug, PartialEq, Eq)]
enum BindingVisual {
    Keys(Vec<String>),
}

impl BindingVisual {
    fn from_binding(binding: &ShortcutBinding) -> Self {
        if binding.input.starts_with("MOUSE")
            || matches!(binding.input.as_str(), "LEFT" | "RIGHT" | "MIDDLE")
        {
            return Self::Keys(vec![mouse_button_label(&binding.input)]);
        }
        let mut keys = binding
            .modifiers
            .iter()
            .map(|modifier| match modifier.as_str() {
                "ctrl" => "Ctrl".to_owned(),
                "alt" => "Alt".to_owned(),
                "shift" => "Shift".to_owned(),
                "win" => "Win".to_owned(),
                other => other.to_owned(),
            })
            .collect::<Vec<_>>();
        keys.push(binding.input.clone());
        Self::Keys(keys)
    }
}

fn mouse_button_label(input: &str) -> String {
    match input {
        "LEFT" => "Left".to_owned(),
        "RIGHT" => "Right".to_owned(),
        "MIDDLE" => "Middle".to_owned(),
        value if value.starts_with("MOUSE") => value.replacen("MOUSE", "MB", 1),
        value => value.to_owned(),
    }
}

const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    COLORREF(red as u32 | ((green as u32) << 8) | ((blue as u32) << 16))
}

fn remaining_seconds(elapsed: f32) -> f32 {
    (READY_CHECK_DURATION_SECS - elapsed).clamp(0.0, READY_CHECK_DURATION_SECS)
}

fn keycap_width(key: &str) -> i32 {
    (key.chars().count() as i32 * 8 + 18).max(32)
}

impl ReadyCheckNotification {
    pub fn new(_owner: HWND, accept: &ShortcutBinding, decline: &ShortcutBinding) -> Result<Self> {
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
        let state = Box::new(NotificationState {
            accept: BindingVisual::from_binding(accept),
            decline: BindingVisual::from_binding(decline),
            timer: Mutex::new(TimerProgress {
                elapsed: 0.0,
                updated_at: Instant::now(),
            }),
        });
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&raw const *state) as isize);
        }
        Ok(Self {
            hwnd,
            _state: state,
        })
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
                let _ = SetTimer(self.hwnd, 1, 100, None);
                let _ = UpdateWindow(self.hwnd);
            }
        } else {
            unsafe {
                let _ = KillTimer(self.hwnd, 1);
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
    }

    pub fn set_timer(&self, elapsed: f32) {
        if let Ok(mut timer) = self._state.timer.lock() {
            timer.elapsed = elapsed.clamp(0.0, READY_CHECK_DURATION_SECS);
            timer.updated_at = Instant::now();
        }
        unsafe {
            let _ = InvalidateRect(self.hwnd, None, false);
        }
    }
}

impl Drop for ReadyCheckNotification {
    fn drop(&mut self) {
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe fn draw_binding(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    area: RECT,
    visual: &BindingVisual,
) {
    let key_fill = rgb(16, 31, 41);
    let shadow = rgb(3, 8, 12);
    match visual {
        BindingVisual::Keys(keys) => {
            let widths = keys.iter().map(|key| keycap_width(key)).collect::<Vec<_>>();
            let total = widths.iter().sum::<i32>() + (widths.len().saturating_sub(1) as i32 * 5);
            let mut left = area.left + ((area.right - area.left - total).max(0) / 2);
            for (index, (key, width)) in keys.iter().zip(widths).enumerate() {
                let right = (left + width).min(area.right);
                let key_rect = RECT {
                    left,
                    top: area.top,
                    right,
                    bottom: area.bottom,
                };
                fill_round_rect(
                    hdc,
                    RECT {
                        top: area.top + 2,
                        bottom: area.bottom + 2,
                        ..key_rect
                    },
                    6,
                    shadow,
                );
                fill_round_rect(hdc, key_rect, 6, key_fill);
                outline_round_rect(
                    hdc,
                    key_rect,
                    6,
                    if index + 1 == keys.len() {
                        rgb(200, 155, 60)
                    } else {
                        rgb(90, 111, 119)
                    },
                    1,
                );
                let mut text = format!("{key}\0").encode_utf16().collect::<Vec<_>>();
                let _ = DrawTextW(
                    hdc,
                    &mut text,
                    &mut RECT {
                        left,
                        right,
                        ..key_rect
                    },
                    DT_CENTER
                        | DT_VCENTER
                        | DT_SINGLELINE
                        | windows::Win32::Graphics::Gdi::DT_NOPREFIX,
                );
                left = right + 5;
            }
        }
    }
}

unsafe fn create_font(height: i32, weight: u32) -> windows::Win32::Graphics::Gdi::HFONT {
    CreateFontW(
        -height,
        0,
        0,
        0,
        weight as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        PROOF_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32 | FF_DONTCARE.0 as u32,
        windows::core::w!("Segoe UI"),
    )
}

unsafe fn draw_label(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    text: &str,
    mut rect: RECT,
    color: COLORREF,
    format: windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT,
) {
    let _ = SetTextColor(hdc, color);
    let mut wide = format!("{text}\0").encode_utf16().collect::<Vec<_>>();
    let _ = DrawTextW(hdc, &mut wide, &mut rect, format);
}

unsafe fn fill_round_rect(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    rect: RECT,
    radius: i32,
    color: COLORREF,
) {
    let brush = CreateSolidBrush(color);
    let region = CreateRoundRectRgn(rect.left, rect.top, rect.right, rect.bottom, radius, radius);
    let _ = FillRgn(hdc, region, HBRUSH(brush.0));
    let _ = DeleteObject(region);
    let _ = DeleteObject(brush);
}

unsafe fn outline_round_rect(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    rect: RECT,
    radius: i32,
    color: COLORREF,
    width: i32,
) {
    let pen = CreatePen(PS_SOLID, width, color);
    let old_pen = SelectObject(hdc, pen);
    let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    let _ = RoundRect(
        hdc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        radius,
        radius,
    );
    let _ = SelectObject(hdc, old_brush);
    let _ = SelectObject(hdc, old_pen);
    let _ = DeleteObject(pen);
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
        let background = CreateSolidBrush(rgb(8, 15, 23));
        let mut rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut rect);
        let _ = FillRect(hdc, &rect, HBRUSH(background.0));
        outline_round_rect(hdc, rect, 18, rgb(79, 67, 40), 1);

        let gold = rgb(200, 155, 60);
        let cyan = rgb(10, 200, 185);
        let white = rgb(240, 230, 210);
        let muted = rgb(148, 163, 172);
        let title_font = create_font(22, FW_SEMIBOLD.0);
        let body_font = create_font(13, FW_NORMAL.0);
        let small_font = create_font(11, FW_SEMIBOLD.0);
        let action_font = create_font(16, FW_SEMIBOLD.0);
        let key_font = create_font(12, FW_SEMIBOLD.0);

        let _ = SetBkMode(hdc, TRANSPARENT);

        let outer_logo = CreateEllipticRgn(20, 18, 66, 64);
        let gold_brush = CreateSolidBrush(gold);
        let _ = FillRgn(hdc, outer_logo, HBRUSH(gold_brush.0));
        let _ = DeleteObject(outer_logo);
        let inner_logo = CreateEllipticRgn(23, 21, 63, 61);
        let inner_brush = CreateSolidBrush(rgb(12, 28, 36));
        let _ = FillRgn(hdc, inner_logo, HBRUSH(inner_brush.0));
        let _ = DeleteObject(inner_logo);
        let check_pen = CreatePen(PS_SOLID, 3, cyan);
        let old_pen = SelectObject(hdc, check_pen);
        let _ = MoveToEx(hdc, 32, 41, None);
        let _ = LineTo(hdc, 40, 49);
        let _ = LineTo(hdc, 55, 32);
        let _ = SelectObject(hdc, old_pen);
        let _ = DeleteObject(check_pen);

        let old_font = SelectObject(hdc, small_font);
        draw_label(
            hdc,
            "LEAGUE READY HOTKEYS",
            RECT {
                left: 80,
                top: 14,
                right: 310,
                bottom: 34,
            },
            gold,
            DT_SINGLELINE | DT_VCENTER,
        );
        let _ = SelectObject(hdc, title_font);
        draw_label(
            hdc,
            "MATCH FOUND",
            RECT {
                left: 80,
                top: 32,
                right: 310,
                bottom: 62,
            },
            white,
            DT_SINGLELINE | DT_VCENTER,
        );
        let _ = SelectObject(hdc, body_font);
        draw_label(
            hdc,
            "Choose an action before time runs out",
            RECT {
                left: 80,
                top: 64,
                right: 330,
                bottom: 88,
            },
            muted,
            DT_SINGLELINE | DT_VCENTER,
        );

        let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const NotificationState;
        if let Some(state) = state.as_ref() {
            let elapsed = state
                .timer
                .lock()
                .map(|timer| timer.elapsed + timer.updated_at.elapsed().as_secs_f32())
                .unwrap_or(0.0)
                .clamp(0.0, READY_CHECK_DURATION_SECS);
            let remaining = remaining_seconds(elapsed);
            fill_round_rect(
                hdc,
                RECT {
                    left: 340,
                    top: 27,
                    right: 400,
                    bottom: 61,
                },
                14,
                rgb(15, 29, 38),
            );
            outline_round_rect(
                hdc,
                RECT {
                    left: 340,
                    top: 27,
                    right: 400,
                    bottom: 61,
                },
                14,
                gold,
                1,
            );
            let _ = SelectObject(hdc, action_font);
            draw_label(
                hdc,
                &format!("{}s", remaining.ceil() as u32),
                RECT {
                    left: 340,
                    top: 27,
                    right: 400,
                    bottom: 61,
                },
                white,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );

            fill_round_rect(hdc, ACCEPT_RECT, 12, rgb(5, 119, 112));
            outline_round_rect(hdc, ACCEPT_RECT, 12, cyan, 1);
            fill_round_rect(hdc, DECLINE_RECT, 12, rgb(12, 23, 32));
            outline_round_rect(hdc, DECLINE_RECT, 12, rgb(93, 110, 120), 1);

            let _ = SelectObject(hdc, action_font);
            draw_label(
                hdc,
                "ACCEPT",
                RECT {
                    left: ACCEPT_RECT.left,
                    top: 110,
                    right: ACCEPT_RECT.right,
                    bottom: 140,
                },
                rgb(238, 255, 252),
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
            draw_label(
                hdc,
                "DECLINE",
                RECT {
                    left: DECLINE_RECT.left,
                    top: 110,
                    right: DECLINE_RECT.right,
                    bottom: 140,
                },
                muted,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );

            let _ = SelectObject(hdc, key_font);
            let _ = SetTextColor(hdc, white);
            draw_binding(
                hdc,
                RECT {
                    left: ACCEPT_RECT.left + 12,
                    top: 143,
                    right: ACCEPT_RECT.right - 12,
                    bottom: 175,
                },
                &state.accept,
            );
            draw_binding(
                hdc,
                RECT {
                    left: DECLINE_RECT.left + 10,
                    top: 143,
                    right: DECLINE_RECT.right - 10,
                    bottom: 175,
                },
                &state.decline,
            );

            let track = RECT {
                left: 20,
                top: 196,
                right: 400,
                bottom: 201,
            };
            fill_round_rect(hdc, track, 4, rgb(30, 42, 49));
            let progress_right = track.left
                + ((track.right - track.left) as f32 * (remaining / READY_CHECK_DURATION_SECS))
                    .round() as i32;
            if progress_right > track.left {
                fill_round_rect(
                    hdc,
                    RECT {
                        right: progress_right,
                        ..track
                    },
                    4,
                    if remaining <= 3.0 {
                        rgb(220, 91, 65)
                    } else {
                        gold
                    },
                );
            }
        }
        let _ = SelectObject(hdc, old_font);
        let _ = DeleteObject(title_font);
        let _ = DeleteObject(body_font);
        let _ = DeleteObject(small_font);
        let _ = DeleteObject(action_font);
        let _ = DeleteObject(key_font);
        let _ = DeleteObject(gold_brush);
        let _ = DeleteObject(inner_brush);
        let _ = DeleteObject(background);
        let _ = EndPaint(hwnd, &paint);
        return LRESULT(0);
    }
    if message == WM_TIMER {
        let _ = InvalidateRect(hwnd, None, false);
        return LRESULT(0);
    }
    if message == WM_SETCURSOR && (lparam.0 & 0xffff) as u32 == HTCLIENT {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    if message == WM_MOUSEMOVE {
        let x = (lparam.0 & 0xffff) as i16 as i32;
        let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
        let on_button = point_in_rect(x, y, ACCEPT_RECT) || point_in_rect(x, y, DECLINE_RECT);
        let cursor_id = if on_button { IDC_HAND } else { IDC_ARROW };
        if let Ok(cursor) = LoadCursorW(None, cursor_id) {
            let _ = SetCursor(cursor);
        }
        return LRESULT(0);
    }
    if message == WM_LBUTTONDOWN {
        let x = (lparam.0 & 0xffff) as i16 as i32;
        let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
        if point_in_rect(x, y, ACCEPT_RECT) {
            println!("notification button clicked: accept");
            ACTION_REQUEST.store(ACTION_ACCEPT, Ordering::Release);
        } else if point_in_rect(x, y, DECLINE_RECT) {
            println!("notification button clicked: decline");
            ACTION_REQUEST.store(ACTION_DECLINE, Ordering::Release);
        } else {
            println!("notification click outside buttons: x={x} y={y}");
        }
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

fn point_in_rect(x: i32, y: i32, rect: RECT) -> bool {
    (rect.left..=rect.right).contains(&x) && (rect.top..=rect.bottom).contains(&y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_binding_becomes_ordered_keycaps() {
        let binding = ShortcutBinding::parse("Ctrl+Shift+A").unwrap();
        assert_eq!(
            BindingVisual::from_binding(&binding),
            BindingVisual::Keys(vec!["Ctrl".into(), "Shift".into(), "A".into()])
        );
    }

    #[test]
    fn mouse_binding_becomes_keycap() {
        let binding = ShortcutBinding::parse("Mouse4").unwrap();
        assert_eq!(
            BindingVisual::from_binding(&binding),
            BindingVisual::Keys(vec!["MB4".into()])
        );
    }

    #[test]
    fn countdown_uses_observed_lcu_timer_range() {
        assert_eq!(remaining_seconds(0.0), 12.0);
        assert_eq!(remaining_seconds(5.0), 7.0);
        assert_eq!(remaining_seconds(12.0), 0.0);
        assert_eq!(remaining_seconds(20.0), 0.0);
    }

    #[test]
    fn keycaps_have_consistent_readable_widths() {
        assert_eq!(keycap_width("P"), 32);
        assert_eq!(keycap_width("MB4"), 42);
        assert_eq!(keycap_width("Shift"), 58);
    }
}
