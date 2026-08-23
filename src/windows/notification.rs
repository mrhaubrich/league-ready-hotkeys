#![cfg(windows)]

use crate::app::HotkeyAction;
use crate::shortcuts::ShortcutBinding;
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::cell::Cell;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows::core::{Error, Result};
use windows::Win32::Foundation::{E_FAIL, HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, SystemParametersInfoW, GWL_EXSTYLE,
    SPI_GETWORKAREA, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

const READY_CHECK_DURATION_SECS: f32 = 12.0;
const IPC_CAPACITY: usize = 8;
const CHILD_EXIT_TIMEOUT: Duration = Duration::from_millis(900);
const WIDTH: i32 = 430;
const HEIGHT: i32 = 238;

slint::slint! {
    export component NotificationWindow inherits Window {
        title: "Ready check";
        width: 430px;
        height: 238px;
        no-frame: true;
        always-on-top: true;
        background: transparent;

        in property<[string]> accept-keys;
        in property<[string]> decline-keys;
        in-out property<float> elapsed: 0;
        callback accept();
        callback decline();

        property<float> remaining: max(0, 12 - root.elapsed);
        property<float> progress: root.remaining / 12;

        Rectangle {
            x: 5px; y: 5px; width: parent.width - 10px; height: parent.height - 10px;
            border-radius: 16px;
            background: #08111a;
            border-width: 1px;
            border-color: #4f4328;

            Rectangle {
                x: 18px; y: 17px; width: 48px; height: 48px;
                border-radius: 24px; background: #0c1c24;
                border-width: 3px; border-color: #c89b3c;
                Text { text: "✓"; color: #0ac8b9; font-size: 28px; font-weight: 700; horizontal-alignment: center; vertical-alignment: center; }
            }
            Text { x: 80px; y: 15px; text: "LEAGUE READY HOTKEYS"; color: #c89b3c; font-size: 12px; font-weight: 700; }
            Text { x: 80px; y: 34px; text: "MATCH FOUND"; color: #f0e6d2; font-size: 23px; font-weight: 700; }
            Text { x: 80px; y: 66px; text: "Choose an action before time runs out"; color: #94a3ac; font-size: 13px; }
            Rectangle {
                x: 345px; y: 24px; width: 57px; height: 34px;
                border-radius: 17px; background: #0f1d26;
                border-width: 1px; border-color: #c89b3c;
                Text { text: ceil(root.remaining) + "s"; color: #f0e6d2; font-size: 16px; font-weight: 700; horizontal-alignment: center; vertical-alignment: center; }
            }

            Rectangle {
                x: 18px; y: 100px; width: 238px; height: 98px;
                border-radius: 11px; background: #057770;
                border-width: 1px; border-color: #0ac8b9;
                TouchArea { clicked => { root.accept(); } }
                Text { y: 10px; width: parent.width; text: "ACCEPT"; color: #eefffc; font-size: 16px; font-weight: 700; horizontal-alignment: center; }
                HorizontalLayout {
                    x: 10px; y: 49px; width: parent.width - 20px; height: 34px; spacing: 5px; alignment: center;
                    for key in root.accept-keys: Rectangle {
                        width: max(34px, accept-key-text.preferred-width + 18px); height: 32px;
                        border-radius: 6px; background: #101f29;
                        border-width: 1px; border-color: #c89b3c;
                        accept-key-text := Text { text: key; color: white; font-size: 12px; font-weight: 700; horizontal-alignment: center; vertical-alignment: center; }
                    }
                }
            }
            Rectangle {
                x: 268px; y: 100px; width: 134px; height: 98px;
                border-radius: 11px; background: #0c1720;
                border-width: 1px; border-color: #5d6e78;
                TouchArea { clicked => { root.decline(); } }
                Text { y: 10px; width: parent.width; text: "DECLINE"; color: #94a3ac; font-size: 16px; font-weight: 700; horizontal-alignment: center; }
                HorizontalLayout {
                    x: 8px; y: 49px; width: parent.width - 16px; height: 34px; spacing: 4px; alignment: center;
                    for key in root.decline-keys: Rectangle {
                        width: max(32px, decline-key-text.preferred-width + 16px); height: 32px;
                        border-radius: 6px; background: #101f29;
                        border-width: 1px; border-color: #5a6f77;
                        decline-key-text := Text { text: key; color: white; font-size: 12px; font-weight: 700; horizontal-alignment: center; vertical-alignment: center; }
                    }
                }
            }
            Rectangle { x: 18px; y: 213px; width: 384px; height: 5px; border-radius: 2px; background: #1e2a31; }
            Rectangle {
                x: 18px; y: 213px; width: 384px * root.progress; height: 5px; border-radius: 2px;
                background: root.remaining <= 3 ? #dc5b41 : #c89b3c;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum ParentCommand {
    Update {
        elapsed: f32,
        accept: Vec<String>,
        decline: Vec<String>,
    },
    DiagnosticAction {
        action: ActionName,
    },
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ActionName {
    Accept,
    Decline,
}

impl From<ActionName> for HotkeyAction {
    fn from(value: ActionName) -> Self {
        match value {
            ActionName::Accept => Self::Accept,
            ActionName::Decline => Self::Decline,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
enum ChildMessage {
    Ready,
    Action { action: ActionName },
}

#[derive(Debug, Default, PartialEq, Eq)]
struct LifecycleState {
    running: bool,
    action_taken: bool,
}

impl LifecycleState {
    fn start(&mut self) -> bool {
        if self.running {
            return false;
        }
        self.running = true;
        self.action_taken = false;
        true
    }

    fn take_action(&mut self, action: ActionName) -> Option<HotkeyAction> {
        if !self.running || self.action_taken {
            return None;
        }
        self.action_taken = true;
        Some(action.into())
    }

    fn stop(&mut self) {
        self.running = false;
        self.action_taken = false;
    }
}

struct NotificationChild {
    process: Child,
    stdin: ChildStdin,
    messages: Receiver<ChildMessage>,
}

impl NotificationChild {
    fn send(&mut self, command: &ParentCommand) -> std::io::Result<()> {
        serde_json::to_writer(&mut self.stdin, command)?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()
    }

    fn terminate(mut self) -> Duration {
        let started = Instant::now();
        let _ = self.send(&ParentCommand::Close);
        let deadline = Instant::now() + CHILD_EXIT_TIMEOUT;
        while Instant::now() < deadline {
            if self.process.try_wait().ok().flatten().is_some() {
                return started.elapsed();
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.process.kill();
        let _ = self.process.wait();
        started.elapsed()
    }
}

struct ControllerState {
    child: Option<NotificationChild>,
    lifecycle: LifecycleState,
    elapsed: f32,
    accept: Vec<String>,
    decline: Vec<String>,
    last_stop_duration: Option<Duration>,
}

pub struct ReadyCheckNotification {
    state: Mutex<ControllerState>,
}

impl ReadyCheckNotification {
    pub fn new(_owner: HWND, accept: &ShortcutBinding, decline: &ShortcutBinding) -> Result<Self> {
        Ok(Self {
            state: Mutex::new(ControllerState {
                child: None,
                lifecycle: LifecycleState::default(),
                elapsed: 0.0,
                accept: binding_keycaps(accept),
                decline: binding_keycaps(decline),
                last_stop_duration: None,
            }),
        })
    }

    pub fn set_active(&self, active: bool) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if active {
            if !state.lifecycle.start() {
                return;
            }
            state.last_stop_duration = None;
            match spawn_child() {
                Ok(mut child) => {
                    let update = current_update(&state);
                    if child.send(&update).is_ok() {
                        state.child = Some(child);
                    } else {
                        state.last_stop_duration = Some(child.terminate());
                        state.lifecycle.stop();
                    }
                }
                Err(error) => {
                    eprintln!("could not start Slint notification child: {error}");
                    state.lifecycle.stop();
                }
            }
        } else {
            state.lifecycle.stop();
            if let Some(child) = state.child.take() {
                state.last_stop_duration = Some(child.terminate());
            }
        }
    }

    pub fn set_timer(&self, elapsed: f32) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.elapsed = clamp_elapsed(elapsed);
        let update = current_update(&state);
        if let Some(child) = state.child.as_mut() {
            let _ = child.send(&update);
        }
    }

    pub fn update_bindings(&self, accept: &ShortcutBinding, decline: &ShortcutBinding) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.accept = binding_keycaps(accept);
        state.decline = binding_keycaps(decline);
        let update = current_update(&state);
        if let Some(child) = state.child.as_mut() {
            let _ = child.send(&update);
        }
    }

    pub fn take_action(&self) -> Option<HotkeyAction> {
        let Ok(mut state) = self.state.lock() else {
            return None;
        };
        loop {
            let message = match state.child.as_ref()?.messages.try_recv() {
                Ok(message) => message,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return None,
            };
            if let ChildMessage::Action { action } = message {
                return state.lifecycle.take_action(action);
            }
        }
    }

    pub fn request_diagnostic_action(&self, action: HotkeyAction) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let command = ParentCommand::DiagnosticAction {
            action: match action {
                HotkeyAction::Accept => ActionName::Accept,
                HotkeyAction::Decline => ActionName::Decline,
            },
        };
        if let Some(child) = state.child.as_mut() {
            let _ = child.send(&command);
        }
    }

    pub fn is_running(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.lifecycle.running)
            .unwrap_or(false)
    }

    pub fn last_stop_duration(&self) -> Option<Duration> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.last_stop_duration)
    }
}

impl Drop for ReadyCheckNotification {
    fn drop(&mut self) {
        if let Ok(state) = self.state.get_mut() {
            state.lifecycle.stop();
            if let Some(child) = state.child.take() {
                state.last_stop_duration = Some(child.terminate());
            }
        }
    }
}

fn current_update(state: &ControllerState) -> ParentCommand {
    ParentCommand::Update {
        elapsed: state.elapsed,
        accept: state.accept.clone(),
        decline: state.decline.clone(),
    }
}

fn spawn_child() -> std::io::Result<NotificationChild> {
    use std::os::windows::process::CommandExt;

    let mut process = Command::new(std::env::current_exe()?)
        .arg("--notification-child")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .creation_flags(0x08000000)
        .spawn()?;
    let stdin = process
        .stdin
        .take()
        .ok_or_else(|| std::io::Error::other("notification child stdin unavailable"))?;
    let stdout = process
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("notification child stdout unavailable"))?;
    let (sender, messages) = mpsc::sync_channel(IPC_CAPACITY);
    std::thread::spawn(move || {
        for line in BufReader::new(stdout)
            .lines()
            .map_while(std::result::Result::ok)
        {
            if let Ok(message) = serde_json::from_str::<ChildMessage>(&line) {
                let _ = sender.try_send(message);
            }
        }
    });
    Ok(NotificationChild {
        process,
        stdin,
        messages,
    })
}

pub fn run_child_process() -> Result<()> {
    use slint::winit_030::{EventResult, WinitWindowAccessor};

    std::env::set_var("SLINT_STYLE", "fluent-dark");
    configure_notification_backend()?;
    let window = NotificationWindow::new().map_err(platform_error)?;
    let styled = Rc::new(Cell::new(false));
    let styled_once = Rc::clone(&styled);
    window
        .window()
        .on_winit_window_event(move |slint_window, _event| {
            if !styled_once.replace(true) {
                if let Err(error) = apply_noactivate_style(slint_window) {
                    eprintln!("could not apply non-activating notification style: {error}");
                    let _ = slint::quit_event_loop();
                } else {
                    write_child_message(&ChildMessage::Ready);
                }
            }
            EventResult::Propagate
        });
    let action_sent = Rc::new(Cell::new(false));
    wire_action(&window, ActionName::Accept, Rc::clone(&action_sent));
    wire_action(&window, ActionName::Decline, action_sent);

    let weak = window.as_weak();
    std::thread::spawn(move || {
        for line in std::io::stdin()
            .lock()
            .lines()
            .map_while(std::result::Result::ok)
        {
            let Ok(command) = serde_json::from_str::<ParentCommand>(&line) else {
                continue;
            };
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || match command {
                ParentCommand::Update {
                    elapsed,
                    accept,
                    decline,
                } => {
                    if let Some(window) = weak.upgrade() {
                        window.set_elapsed(clamp_elapsed(elapsed));
                        window.set_accept_keys(string_model(accept));
                        window.set_decline_keys(string_model(decline));
                    }
                }
                ParentCommand::DiagnosticAction { action } => {
                    if let Some(window) = weak.upgrade() {
                        match action {
                            ActionName::Accept => window.invoke_accept(),
                            ActionName::Decline => window.invoke_decline(),
                        }
                    }
                }
                ParentCommand::Close => {
                    let _ = slint::quit_event_loop();
                }
            });
        }
        let _ = slint::invoke_from_event_loop(|| {
            let _ = slint::quit_event_loop();
        });
    });

    let timer = Timer::default();
    let weak = window.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
        if let Some(window) = weak.upgrade() {
            window.set_elapsed(clamp_elapsed(window.get_elapsed() + 0.1));
        }
    });
    window.show().map_err(platform_error)?;
    slint::run_event_loop().map_err(platform_error)
}

fn wire_action(window: &NotificationWindow, action: ActionName, sent: Rc<Cell<bool>>) {
    let callback = move || {
        if sent.replace(true) {
            return;
        }
        write_child_message(&ChildMessage::Action { action });
    };
    match action {
        ActionName::Accept => window.on_accept(callback),
        ActionName::Decline => window.on_decline(callback),
    }
}

fn write_child_message(message: &ChildMessage) {
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    if serde_json::to_writer(&mut output, message).is_ok() {
        let _ = output.write_all(b"\n");
        let _ = output.flush();
    }
}

fn configure_notification_backend() -> Result<()> {
    use slint::winit_030::winit::dpi::PhysicalPosition;
    use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
    use slint::winit_030::winit::window::WindowLevel;

    let mut work_area = RECT::default();
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&mut work_area as *mut RECT).cast()),
            Default::default(),
        )?;
    }
    let position =
        PhysicalPosition::new(work_area.right - WIDTH - 20, work_area.bottom - HEIGHT - 20);
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .with_winit_window_attributes_hook(move |attributes| {
            attributes
                .with_active(false)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_position(position)
                .with_skip_taskbar(true)
        })
        .select()
        .map_err(platform_error)
}

fn apply_noactivate_style(window: &slint::Window) -> Result<()> {
    use slint::winit_030::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use slint::winit_030::WinitWindowAccessor;

    window
        .with_winit_window(|winit_window| {
            let raw = winit_window
                .window_handle()
                .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
            let RawWindowHandle::Win32(win32) = raw.as_raw() else {
                return Err(Error::new(E_FAIL, "notification window is not Win32"));
            };
            let hwnd = HWND(win32.hwnd.get() as *mut std::ffi::c_void);
            unsafe {
                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                SetWindowLongPtrW(
                    hwnd,
                    GWL_EXSTYLE,
                    (ex_style | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize,
                );
                SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
                )?;
            }
            Ok(())
        })
        .ok_or_else(|| Error::new(E_FAIL, "Slint did not create a winit notification window"))?
}

fn binding_keycaps(binding: &ShortcutBinding) -> Vec<String> {
    if binding.input.starts_with("MOUSE") {
        return vec![binding.input.replacen("MOUSE", "MB", 1)];
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
    keys
}

fn string_model(values: Vec<String>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        values
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
}

fn clamp_elapsed(elapsed: f32) -> f32 {
    if elapsed.is_finite() {
        elapsed.clamp(0.0, READY_CHECK_DURATION_SECS)
    } else {
        0.0
    }
}

fn platform_error(error: slint::PlatformError) -> Error {
    Error::new(E_FAIL, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_commands_round_trip() {
        let command = ParentCommand::Update {
            elapsed: 4.5,
            accept: vec!["Ctrl".into(), "P".into()],
            decline: vec!["MB4".into()],
        };
        let json = serde_json::to_string(&command).unwrap();
        assert_eq!(
            serde_json::from_str::<ParentCommand>(&json).unwrap(),
            command
        );
        let action = ChildMessage::Action {
            action: ActionName::Decline,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(serde_json::from_str::<ChildMessage>(&json).unwrap(), action);
    }

    #[test]
    fn formats_keyboard_and_mouse_keycaps() {
        assert_eq!(
            binding_keycaps(&ShortcutBinding::parse("Ctrl+Shift+P").unwrap()),
            ["Ctrl", "Shift", "P"]
        );
        assert_eq!(
            binding_keycaps(&ShortcutBinding::parse("Mouse4").unwrap()),
            ["MB4"]
        );
    }

    #[test]
    fn timer_is_clamped_to_observed_contract() {
        assert_eq!(clamp_elapsed(-2.0), 0.0);
        assert_eq!(clamp_elapsed(5.5), 5.5);
        assert_eq!(clamp_elapsed(99.0), 12.0);
        assert_eq!(clamp_elapsed(f32::NAN), 0.0);
    }

    #[test]
    fn lifecycle_suppresses_duplicate_start_and_action() {
        let mut state = LifecycleState::default();
        assert!(state.start());
        assert!(!state.start());
        assert_eq!(
            state.take_action(ActionName::Accept),
            Some(HotkeyAction::Accept)
        );
        assert_eq!(state.take_action(ActionName::Decline), None);
        state.stop();
        assert!(!state.running);
        assert!(state.start());
        assert_eq!(
            state.take_action(ActionName::Decline),
            Some(HotkeyAction::Decline)
        );
    }
}
