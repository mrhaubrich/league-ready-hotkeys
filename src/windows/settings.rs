#![cfg(windows)]

use crate::shortcuts::ShortcutBinding;
use slint::{CloseRequestResponse, ComponentHandle, SharedString, Timer, TimerMode};
use std::{cell::RefCell, rc::Rc, time::Duration};
use windows::core::{Error, Result};
use windows::Win32::Foundation::{E_FAIL, HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

pub const SETTINGS_UPDATED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 3;
pub const SETTINGS_CLOSED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 4;

slint::slint! {
    import { Button, HorizontalBox, VerticalBox } from "std-widgets.slint";
    export component SettingsWindow inherits Window {
        title: "Configure hotkeys"; width: 460px; height: 390px;
        background: #07111b;
        in property<string> accept-binding;
        in property<string> decline-binding;
        in property<string> status-message;
        in property<bool> dirty;
        callback configure-accept(); callback configure-decline(); callback save(); callback cancel();
        VerticalBox {
            padding: 28px; spacing: 16px;
            Text { text: "LEAGUE READY HOTKEYS"; color: #c89b3c; font-size: 13px; font-weight: 700; }
            Text { text: "Configure hotkeys"; color: #f0e6d2; font-size: 25px; font-weight: 700; }
            Text { text: root.status-message; color: #9aa7b4; font-size: 14px; }
            Rectangle {
                height: 76px; border-radius: 9px; background: #0d1c28; border-width: 1px; border-color: #274052;
                TouchArea { clicked => { root.configure-accept(); } }
                HorizontalLayout { padding-left: 18px; padding-right: 18px;
                    Rectangle { horizontal-stretch: 1;
                        Text { text: "Accept"; color: #f0e6d2; font-size: 17px; horizontal-alignment: left; vertical-alignment: center; }
                    }
                    Rectangle { width: 110px; height: 38px; border-radius: 6px; background: #09131d; border-width: 1px; border-color: #0ac8b9;
                        Text { text: root.accept-binding; color: white; font-size: 15px; font-weight: 700; horizontal-alignment: center; vertical-alignment: center; }
                    }
                    Rectangle { horizontal-stretch: 1; }
                }
            }
            Rectangle {
                height: 76px; border-radius: 9px; background: #0d1c28; border-width: 1px; border-color: #274052;
                TouchArea { clicked => { root.configure-decline(); } }
                HorizontalLayout { padding-left: 18px; padding-right: 18px;
                    Rectangle { horizontal-stretch: 1;
                        Text { text: "Decline"; color: #f0e6d2; font-size: 17px; horizontal-alignment: left; vertical-alignment: center; }
                    }
                    Rectangle { width: 110px; height: 38px; border-radius: 6px; background: #09131d; border-width: 1px; border-color: #5b7182;
                        Text { text: root.decline-binding; color: white; font-size: 15px; font-weight: 700; horizontal-alignment: center; vertical-alignment: center; }
                    }
                    Rectangle { horizontal-stretch: 1; }
                }
            }
            Rectangle { vertical-stretch: 1; }
            HorizontalBox { spacing: 12px; Rectangle { horizontal-stretch: 1; }
                Button { text: "Cancel"; clicked => { root.cancel(); } }
                Button { text: root.dirty ? "Save changes" : "Saved"; enabled: root.dirty; clicked => { root.save(); } }
            }
        }
    }
}

struct State {
    owner: HWND,
    accept: ShortcutBinding,
    decline: ShortcutBinding,
    target: u8,
    previous: [bool; 256],
}
pub struct HotkeySettings {
    child: RefCell<std::process::Child>,
}

impl HotkeySettings {
    pub fn new(owner: HWND) -> Result<Self> {
        use std::os::windows::process::CommandExt;
        let child = std::process::Command::new(std::env::current_exe().map_err(io_error)?)
            .arg("--settings-window")
            .arg((owner.0 as usize).to_string())
            .creation_flags(0x08000000)
            .spawn()
            .map_err(io_error)?;
        Ok(Self {
            child: RefCell::new(child),
        })
    }
    pub fn show(&self) {}
}

impl Drop for HotkeySettings {
    fn drop(&mut self) {
        let child = self.child.get_mut();
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn run_window_process(owner_value: usize) -> Result<()> {
    std::env::set_var("SLINT_STYLE", "fluent-dark");
    let owner = HWND(owner_value as *mut std::ffi::c_void);
    let window = SettingsWindow::new().map_err(platform_error)?;
    let (accept, decline) = crate::windows::startup::load_bindings();
    let state = Rc::new(RefCell::new(State {
        owner,
        accept,
        decline,
        target: 0,
        previous: [false; 256],
    }));
    refresh(
        &window,
        &state.borrow(),
        "Choose an action to change its shortcut",
        false,
    );
    let timer = Timer::default();
    wire_capture(&window, &state, &timer);
    wire_actions(&window, &state);
    window.window().on_close_requested(move || {
        unsafe {
            let _ = PostMessageW(owner, SETTINGS_CLOSED, WPARAM(0), LPARAM(0));
        }
        let _ = slint::quit_event_loop();
        CloseRequestResponse::HideWindow
    });
    window.show().map_err(platform_error)?;
    slint::run_event_loop().map_err(platform_error)
}

fn platform_error(error: slint::PlatformError) -> Error {
    Error::new(E_FAIL, error.to_string())
}
fn io_error(error: std::io::Error) -> Error {
    Error::new(E_FAIL, error.to_string())
}

fn wire_capture(window: &SettingsWindow, state: &Rc<RefCell<State>>, timer: &Timer) {
    let weak = window.as_weak();
    let polling = Rc::clone(state);
    timer.start(TimerMode::Repeated, Duration::from_millis(30), move || {
        let Some(window) = weak.upgrade() else { return };
        let mut state = polling.borrow_mut();
        if state.target == 0 {
            return;
        }
        for vk in candidates() {
            let down = unsafe { GetAsyncKeyState(vk as i32) < 0 };
            let old = state.previous[vk as usize];
            state.previous[vk as usize] = down;
            if !down || old {
                continue;
            }
            let Some(value) = candidate_binding(vk) else {
                continue;
            };
            let Ok(binding) = ShortcutBinding::parse(&value) else {
                continue;
            };
            if (state.target == 1 && binding == state.decline)
                || (state.target == 2 && binding == state.accept)
            {
                state.target = 0;
                window.set_status_message("Shortcuts must be different".into());
                return;
            }
            if state.target == 1 {
                state.accept = binding;
            } else {
                state.decline = binding;
            }
            state.target = 0;
            refresh(&window, &state, "Unsaved changes", true);
            return;
        }
    });
    let bind = |target, weak: slint::Weak<SettingsWindow>, state: Rc<RefCell<State>>| {
        move || {
            let mut state = state.borrow_mut();
            state.target = target;
            for vk in candidates() {
                state.previous[vk as usize] = unsafe { GetAsyncKeyState(vk as i32) < 0 };
            }
            if let Some(w) = weak.upgrade() {
                w.set_status_message("Listening… press a keyboard key or mouse button".into());
            }
        }
    };
    window.on_configure_accept(bind(1, window.as_weak(), Rc::clone(state)));
    window.on_configure_decline(bind(2, window.as_weak(), Rc::clone(state)));
}

fn wire_actions(window: &SettingsWindow, state: &Rc<RefCell<State>>) {
    let weak = window.as_weak();
    let saved = Rc::clone(state);
    window.on_save(move || {
        let state = saved.borrow();
        if crate::windows::startup::save_bindings(
            &state.accept.canonical(),
            &state.decline.canonical(),
        )
        .is_ok()
        {
            if let Some(w) = weak.upgrade() {
                refresh(&w, &state, "Hotkeys saved", false);
            }
            unsafe {
                let _ = PostMessageW(state.owner, SETTINGS_UPDATED, WPARAM(0), LPARAM(0));
            }
        }
    });
    let weak = window.as_weak();
    let cancelled = Rc::clone(state);
    window.on_cancel(move || {
        let (accept, decline) = crate::windows::startup::load_bindings();
        let mut state = cancelled.borrow_mut();
        state.accept = accept;
        state.decline = decline;
        state.target = 0;
        if let Some(w) = weak.upgrade() {
            refresh(&w, &state, "Changes cancelled", false);
        }
    });
}

fn refresh(window: &SettingsWindow, state: &State, message: &str, dirty: bool) {
    window.set_accept_binding(SharedString::from(label(&state.accept)));
    window.set_decline_binding(SharedString::from(label(&state.decline)));
    window.set_status_message(message.into());
    window.set_dirty(dirty);
}
fn label(binding: &ShortcutBinding) -> String {
    binding.canonical().replace("MOUSE", "MB")
}
fn down(vk: i32) -> bool {
    unsafe { GetAsyncKeyState(vk) < 0 }
}
fn candidates() -> impl Iterator<Item = u32> {
    (0x30..=0x39)
        .chain(0x41..=0x5a)
        .chain(0x70..=0x7b)
        .chain([1, 2, 4, 5])
}
fn candidate_binding(vk: u32) -> Option<String> {
    let mods = [(0x11, "Ctrl"), (0x12, "Alt"), (0x10, "Shift")]
        .into_iter()
        .filter(|(k, _)| down(*k))
        .map(|(_, n)| n)
        .chain(if down(0x5b) || down(0x5c) {
            Some("Win")
        } else {
            None
        })
        .collect::<Vec<_>>();
    let input = match vk {
        1 => "Mouse1".into(),
        2 => "Mouse2".into(),
        4 => "Mouse4".into(),
        5 => "Mouse5".into(),
        0x70..=0x7b => format!("F{}", vk - 0x6f),
        0x30..=0x39 | 0x41..=0x5a => char::from_u32(vk)?.to_string(),
        _ => return None,
    };
    Some(if mods.is_empty() {
        input
    } else {
        format!("{}+{input}", mods.join("+"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labels_mouse() {
        assert_eq!(label(&ShortcutBinding::parse("Mouse4").unwrap()), "MB4");
    }
    #[test]
    fn captures_inputs() {
        assert_eq!(candidate_binding(0x70).as_deref(), Some("F1"));
        assert_eq!(candidate_binding(4).as_deref(), Some("Mouse4"));
    }
}
