#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str);
    if mode == Some("--check-hotkeys") {
        run_hotkey_diagnostic();
        return;
    }
    if mode == Some("--check-tray") {
        run_tray_diagnostic();
        return;
    }
    if matches!(mode, Some("--check-startup") | Some("--enable-startup") | Some("--disable-startup")) {
        let executable = std::env::current_exe().expect("current executable path");
        let result = match mode {
            Some("--check-startup") => league_ready_hotkeys::windows::startup::is_enabled(),
            Some("--enable-startup") => league_ready_hotkeys::windows::startup::set_enabled(&executable, true).map(|_| true),
            Some("--disable-startup") => league_ready_hotkeys::windows::startup::set_enabled(&executable, false).map(|_| false),
            _ => unreachable!(),
        };
        match result { Ok(enabled) => println!("startup enabled: {enabled}"), Err(error) => { eprintln!("startup registry operation failed: {error}"); std::process::exit(1); } }
        return;
    }
    if !matches!(mode, Some("--check-lockfile") | Some("--watch-ready-check") | Some("--check-action")) {
        println!("use --check-lockfile, --watch-ready-check, or --check-action accept|decline");
        return;
    }
    let action = if mode == Some("--check-action") { args.get(2).map(String::as_str) } else { None };
    if mode == Some("--check-action") && !matches!(action, Some("accept") | Some("decline")) {
        eprintln!("usage: --check-action accept|decline");
        std::process::exit(2);
    }
    let path_arg = if action.is_some() { args.get(3) } else { args.get(2) };
    let path = path_arg.cloned().or_else(|| league_ready_hotkeys::lcu::discover_lockfile().ok().map(|path| path.to_string_lossy().into_owned()));
    let Some(path) = path else {
        eprintln!("LeagueClientUx.exe is not running or its lockfile is unavailable");
        std::process::exit(1);
    };
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => { eprintln!("could not read lockfile: {error}"); std::process::exit(1); }
    };
    let credentials = match league_ready_hotkeys::lcu::parse_lockfile(&contents) {
        Ok(credentials) => credentials,
        Err(error) => { eprintln!("invalid lockfile: {error}"); std::process::exit(1); }
    };
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()
        .expect("create diagnostic runtime");
    let result = runtime.block_on(async {
        let client = league_ready_hotkeys::lcu::transport::LcuClient::new(&credentials)
            .map_err(|error| error.to_string())?;
        if mode == Some("--watch-ready-check") {
            let event = client.next_ready_check_event().await.map_err(|error| error.to_string())?;
            return Ok(Some(serde_json::json!({"active": event.active, "response": format!("{:?}", event.response)})));
        }
        let ready = client.ready_check().await.map_err(|error| error.to_string())?;
        if let Some(action) = action {
            let payload = ready.ok_or_else(|| "no active ready check".to_owned())?;
            let state = league_ready_hotkeys::lcu::parse_ready_check(&payload).map_err(|error| error.to_string())?;
            if !state.active { return Err("ready check is not active or already answered".to_owned()); }
            match action { "accept" => client.accept().await, "decline" => client.decline().await, _ => unreachable!() }
                .map_err(|error| error.to_string())?;
            return Ok(Some(serde_json::json!({"action": action, "sent": true})));
        }
        Ok(ready)
    });
    match result {
        Ok(Some(payload)) if action.is_some() => println!("LCU action sent successfully: {payload}"),
        Ok(Some(payload)) => println!("LCU reachable; ready-check response: {payload}"),
        Ok(None) => println!("LCU reachable; no active ready check (HTTP 404)"),
        Err(error) => { eprintln!("LCU request failed: {error}"); std::process::exit(1); }
    }
}

#[cfg(windows)]
fn run_hotkey_diagnostic() {
use std::time::{Duration, Instant};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{PeekMessageW, MSG, PM_REMOVE, WM_HOTKEY};
    let mut manager = league_ready_hotkeys::windows::hotkeys::HotkeyManager::new(HWND(std::ptr::null_mut()));
    if let Err(error) = manager.set_enabled(true) {
        eprintln!("could not register F1/F2: {error}");
        std::process::exit(1);
    }
    println!("F1/F2 diagnostic active for 30 seconds; press either key (they are temporarily captured globally)");
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE) }.as_bool() {
            if message.message == WM_HOTKEY {
                match message.wParam.0 as i32 {
                    league_ready_hotkeys::windows::hotkeys::ACCEPT_HOTKEY_ID => println!("received F1 (accept binding)"),
                    league_ready_hotkeys::windows::hotkeys::DECLINE_HOTKEY_ID => println!("received F2 (decline binding)"),
                    _ => println!("received unknown hotkey id {}", message.wParam.0),
                }
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    println!("F1/F2 diagnostic complete; bindings released");
}

#[cfg(windows)]
fn run_tray_diagnostic() {
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, PeekMessageW, RegisterClassW, MSG, PM_REMOVE, HWND_MESSAGE, WS_OVERLAPPED, WM_RBUTTONUP, WM_APP, WNDCLASSW, CS_HREDRAW, CS_VREDRAW};
    let class_name = windows::core::w!("LeagueReadyHotkeysTrayWindow");
    unsafe { let _ = RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(tray_wnd_proc), hInstance: Default::default(), lpszClassName: class_name, style: CS_HREDRAW | CS_VREDRAW, ..Default::default() }); }
    let owner = unsafe { CreateWindowExW(Default::default(), class_name, windows::core::w!("League Ready Hotkeys"), WS_OVERLAPPED, 0, 0, 0, 0, HWND_MESSAGE, None, None, None) }
        .unwrap_or_else(|error| { eprintln!("could not create tray message window: {error}"); std::process::exit(1); });
    let icon_path = std::path::Path::new("assets\\tray-icon.ico");
    let mut tray = league_ready_hotkeys::windows::tray::TrayIcon::with_icon(owner, icon_path)
        .unwrap_or_else(|error| { eprintln!("could not load tray icon: {error}"); std::process::exit(1); });
    if !tray.add() {
        eprintln!("could not add tray icon");
        std::process::exit(1);
    }
    println!("tray icon active for 30 seconds; right-click it to open the menu");
    let deadline = Instant::now() + Duration::from_secs(30);
    'tray: while Instant::now() < deadline {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE) }.as_bool() {
            if message.message == WM_APP + 2 {
                break 'tray;
            }
            if message.message == WM_APP + 1 {
                println!("tray callback received: event={} expected={}", message.lParam.0, WM_RBUTTONUP);
                if message.lParam.0 as u32 == WM_RBUTTONUP && tray.show_menu() { break 'tray; }
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    tray.remove();
    unsafe { let _ = DestroyWindow(owner); }
    println!("tray icon removed");
}

#[cfg(windows)]
static TRAY_EXIT_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
unsafe extern "system" fn tray_wnd_proc(hwnd: windows::Win32::Foundation::HWND, message: u32, wparam: windows::Win32::Foundation::WPARAM, lparam: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::LRESULT {
    if message == league_ready_hotkeys::windows::tray::TRAY_MESSAGE {
        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(hwnd, windows::Win32::UI::WindowsAndMessaging::WM_APP + 1, wparam, lparam);
    } else if message == windows::Win32::UI::WindowsAndMessaging::WM_COMMAND && (wparam.0 & 0xffff) as u32 == league_ready_hotkeys::windows::tray::MENU_EXIT {
        TRAY_EXIT_REQUESTED.store(true, std::sync::atomic::Ordering::Release);
    }
    windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, message, wparam, lparam)
}

#[cfg(not(windows))]
fn main() {
    eprintln!("league-ready-hotkeys is supported on Windows only");
}
