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
    if mode == Some("--check-notification") {
        run_notification_diagnostic();
        return;
    }
    if mode == Some("--check-shortcuts") {
        let config = league_ready_hotkeys::windows::startup::load_shortcuts();
        println!(
            "shortcuts: accept={:?} decline={:?}",
            config.accept, config.decline
        );
        return;
    }
    if mode == Some("--set-shortcuts") {
        let config = league_ready_hotkeys::shortcuts::ShortcutConfig::parse(
            args.get(2).map(String::as_str).unwrap_or_default(),
            args.get(3).map(String::as_str).unwrap_or_default(),
        )
        .unwrap_or_else(|error| {
            eprintln!("invalid shortcuts: {error}");
            std::process::exit(2);
        });
        if let Err(error) = league_ready_hotkeys::windows::startup::save_shortcuts(config) {
            eprintln!("could not save shortcuts: {error}");
            std::process::exit(1);
        }
        println!(
            "shortcuts saved: accept={:?} decline={:?}",
            config.accept, config.decline
        );
        return;
    }
    if mode == Some("--check-binding") {
        let value = args.get(2).map(String::as_str).unwrap_or_default();
        match league_ready_hotkeys::shortcuts::ShortcutBinding::parse(value) {
            Ok(binding) => println!(
                "valid binding: modifiers={:?} input={}",
                binding.modifiers, binding.input
            ),
            Err(error) => {
                eprintln!("invalid binding: {error}");
                std::process::exit(2);
            }
        }
        return;
    }
    if mode == Some("--check-input-hook") {
        league_ready_hotkeys::windows::input_hooks::run_diagnostic();
        return;
    }
    if matches!(
        mode,
        Some("--check-startup") | Some("--enable-startup") | Some("--disable-startup")
    ) {
        let executable = std::env::current_exe().expect("current executable path");
        let result = match mode {
            Some("--check-startup") => league_ready_hotkeys::windows::startup::is_enabled(),
            Some("--enable-startup") => {
                league_ready_hotkeys::windows::startup::set_enabled(&executable, true).map(|_| true)
            }
            Some("--disable-startup") => {
                league_ready_hotkeys::windows::startup::set_enabled(&executable, false)
                    .map(|_| false)
            }
            _ => unreachable!(),
        };
        match result {
            Ok(enabled) => println!("startup enabled: {enabled}"),
            Err(error) => {
                eprintln!("startup registry operation failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if mode.is_none() {
        run_background();
        return;
    }
    if !matches!(
        mode,
        Some("--check-lockfile") | Some("--watch-ready-check") | Some("--check-action")
    ) {
        println!("use --check-lockfile, --watch-ready-check, or --check-action accept|decline");
        return;
    }
    let action = if mode == Some("--check-action") {
        args.get(2).map(String::as_str)
    } else {
        None
    };
    if mode == Some("--check-action") && !matches!(action, Some("accept") | Some("decline")) {
        eprintln!("usage: --check-action accept|decline");
        std::process::exit(2);
    }
    let path_arg = if action.is_some() {
        args.get(3)
    } else {
        args.get(2)
    };
    let path = path_arg.cloned().or_else(|| {
        league_ready_hotkeys::lcu::discover_lockfile()
            .ok()
            .map(|path| path.to_string_lossy().into_owned())
    });
    let Some(path) = path else {
        eprintln!("LeagueClientUx.exe is not running or its lockfile is unavailable");
        std::process::exit(1);
    };
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("could not read lockfile: {error}");
            std::process::exit(1);
        }
    };
    let credentials = match league_ready_hotkeys::lcu::parse_lockfile(&contents) {
        Ok(credentials) => credentials,
        Err(error) => {
            eprintln!("invalid lockfile: {error}");
            std::process::exit(1);
        }
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("create diagnostic runtime");
    let result = runtime.block_on(async {
        let client = league_ready_hotkeys::lcu::transport::LcuClient::new(&credentials)
            .map_err(|error| error.to_string())?;
        if mode == Some("--watch-ready-check") {
            let event = client.next_ready_check_event_with_retry(5).await.map_err(|error| error.to_string())?;
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
        Ok(Some(payload)) if action.is_some() => {
            println!("LCU action sent successfully: {payload}")
        }
        Ok(Some(payload)) => println!("LCU reachable; ready-check response: {payload}"),
        Ok(None) => println!("LCU reachable; no active ready check (HTTP 404)"),
        Err(error) => {
            eprintln!("LCU request failed: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(windows)]
fn run_background() {
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, DispatchMessageW, PeekMessageW, RegisterClassW, CS_HREDRAW,
        CS_VREDRAW, HWND_MESSAGE, MSG, PM_REMOVE, WM_APP, WM_HOTKEY, WM_RBUTTONUP, WNDCLASSW,
        WS_OVERLAPPED,
    };
    let class_name = windows::core::w!("LeagueReadyHotkeysBackgroundWindow");
    unsafe {
        let _ = RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: Default::default(),
            lpszClassName: class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        });
    }
    let owner = unsafe {
        CreateWindowExW(
            Default::default(),
            class_name,
            windows::core::w!("League Ready Hotkeys"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            None,
            None,
        )
    }
    .unwrap_or_else(|error| {
        eprintln!("could not create background window: {error}");
        std::process::exit(1);
    });
    let icon_path = tray_icon_path();
    let mut tray = league_ready_hotkeys::windows::tray::TrayIcon::with_icon(owner, &icon_path)
        .unwrap_or_else(|error| {
            eprintln!("could not load tray icon: {error}");
            std::process::exit(1);
        });
    if !tray.add() {
        eprintln!("could not add tray icon");
        std::process::exit(1);
    }
    let mut hotkeys = league_ready_hotkeys::windows::hotkeys::HotkeyManager::new(owner);
    let shortcut_config = league_ready_hotkeys::windows::startup::load_shortcuts();
    let notification =
        league_ready_hotkeys::windows::notification::ReadyCheckNotification::new(owner)
            .unwrap_or_else(|error| {
                eprintln!("could not create notification: {error}");
                std::process::exit(1);
            });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("create background runtime");
    let mut client: Option<league_ready_hotkeys::lcu::transport::LcuClient> = None;
    let mut active = false;
    let mut next_poll = Instant::now();
    println!(
        "League Ready Hotkeys running in background; F1/F2 activate only during a ready check"
    );
    'background: loop {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE) }
            .as_bool()
        {
            if message.message == WM_APP + 1 && message.lParam.0 as u32 == WM_RBUTTONUP {
                let startup_enabled =
                    league_ready_hotkeys::windows::startup::is_enabled().unwrap_or(false);
                let command = tray.show_menu(
                    startup_enabled,
                    league_ready_hotkeys::windows::startup::notifications_enabled().unwrap_or(true),
                );
                if command == league_ready_hotkeys::windows::tray::MENU_EXIT
                    || TRAY_EXIT_REQUESTED.swap(false, std::sync::atomic::Ordering::AcqRel)
                {
                    break 'background;
                }
                if command == league_ready_hotkeys::windows::tray::MENU_STARTUP
                    || STARTUP_TOGGLE_REQUESTED.swap(false, std::sync::atomic::Ordering::AcqRel)
                {
                    if let Ok(enabled) = league_ready_hotkeys::windows::startup::is_enabled() {
                        let executable = std::env::current_exe().expect("current executable path");
                        let _ = league_ready_hotkeys::windows::startup::set_enabled(
                            &executable,
                            !enabled,
                        );
                    }
                }
                if command == league_ready_hotkeys::windows::tray::MENU_NOTIFICATIONS {
                    if let Ok(enabled) =
                        league_ready_hotkeys::windows::startup::notifications_enabled()
                    {
                        let _ = league_ready_hotkeys::windows::startup::set_notifications_enabled(
                            !enabled,
                        );
                    }
                }
            }
            if message.message == WM_APP + 2 {
                break 'background;
            }
            if message.message == WM_HOTKEY && active {
                let action = match message.wParam.0 as i32 {
                    league_ready_hotkeys::windows::hotkeys::ACCEPT_HOTKEY_ID => Some("accept"),
                    league_ready_hotkeys::windows::hotkeys::DECLINE_HOTKEY_ID => Some("decline"),
                    _ => None,
                };
                if let (Some(action), Some(lcu)) = (action, client.as_ref()) {
                    let result = runtime.block_on(async {
                        if action == "accept" {
                            lcu.accept().await
                        } else {
                            lcu.decline().await
                        }
                    });
                    if let Err(error) = result {
                        eprintln!("LCU action failed: {error}");
                    }
                    active = false;
                    let _ = hotkeys.set_enabled(false);
                }
            }
            unsafe {
                DispatchMessageW(&message);
            }
        }
        let requested = league_ready_hotkeys::windows::notification::take_action();
        if active && requested != league_ready_hotkeys::windows::notification::ACTION_NONE {
            if let Some(lcu) = client.as_ref() {
                let result = runtime.block_on(async {
                    if requested == league_ready_hotkeys::windows::notification::ACTION_ACCEPT {
                        lcu.accept().await
                    } else {
                        lcu.decline().await
                    }
                });
                if let Err(error) = result {
                    eprintln!("notification action failed: {error}");
                }
                active = false;
                let _ = hotkeys.set_enabled(false);
                notification.set_active(false);
            }
        }
        if Instant::now() >= next_poll {
            next_poll = Instant::now() + Duration::from_secs(2);
            if let Ok(path) = league_ready_hotkeys::lcu::discover_lockfile() {
                if let Ok(contents) = std::fs::read_to_string(path) {
                    if let Ok(credentials) = league_ready_hotkeys::lcu::parse_lockfile(&contents) {
                        if let Ok(lcu) =
                            league_ready_hotkeys::lcu::transport::LcuClient::new(&credentials)
                        {
                            client = Some(lcu);
                        }
                    }
                }
            } else {
                client = None;
            }
            let ready = if let Some(lcu) = client.as_ref() {
                runtime
                    .block_on(lcu.ready_check())
                    .ok()
                    .flatten()
                    .and_then(|payload| league_ready_hotkeys::lcu::parse_ready_check(&payload).ok())
                    .map(|state| state.active)
                    .unwrap_or(false)
            } else {
                false
            };
            if ready != active {
                active = ready;
                let _ = hotkeys.set_enabled_with_config(active, shortcut_config);
                notification.set_active(
                    active
                        && league_ready_hotkeys::windows::startup::notifications_enabled()
                            .unwrap_or(true),
                );
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = hotkeys.set_enabled(false);
    tray.remove();
    unsafe {
        let _ = DestroyWindow(owner);
    }
    println!("background utility stopped");
}

#[cfg(windows)]
fn run_notification_diagnostic() {
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, DispatchMessageW, PeekMessageW, RegisterClassW, CS_HREDRAW,
        CS_VREDRAW, HWND_MESSAGE, MSG, PM_REMOVE, WNDCLASSW, WS_OVERLAPPED,
    };
    let class_name = windows::core::w!("LeagueReadyHotkeysNotificationDiagnostic");
    unsafe {
        let _ = RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: Default::default(),
            lpszClassName: class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        });
    }
    let owner = unsafe {
        CreateWindowExW(
            Default::default(),
            class_name,
            windows::core::w!("League Ready Hotkeys"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            None,
            None,
        )
    }
    .unwrap_or_else(|error| {
        eprintln!("could not create notification window: {error}");
        std::process::exit(1);
    });
    let notification =
        league_ready_hotkeys::windows::notification::ReadyCheckNotification::new(owner)
            .unwrap_or_else(|error| {
                eprintln!("could not create notification: {error}");
                std::process::exit(1);
            });
    notification.set_active(true);
    println!("notification diagnostic active for 30 seconds");
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE) }
            .as_bool()
        {
            unsafe {
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    notification.set_active(false);
    unsafe {
        let _ = DestroyWindow(owner);
    }
    println!("notification diagnostic complete");
}

#[cfg(windows)]
fn tray_icon_path() -> std::path::PathBuf {
    let relative = std::path::Path::new("assets\\tray-icon.ico");
    let mut candidates = vec![relative.to_path_buf()];
    if let Ok(executable) = std::env::current_exe() {
        if let Some(bin_dir) = executable.parent() {
            candidates.push(bin_dir.join(relative));
            if let Some(project_dir) = bin_dir.parent().and_then(|p| p.parent()) {
                candidates.push(project_dir.join(relative));
            }
        }
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| relative.to_path_buf())
}

#[cfg(windows)]
fn run_hotkey_diagnostic() {
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{PeekMessageW, MSG, PM_REMOVE, WM_HOTKEY};
    let mut manager =
        league_ready_hotkeys::windows::hotkeys::HotkeyManager::new(HWND(std::ptr::null_mut()));
    if let Err(error) = manager.set_enabled(true) {
        eprintln!("could not register F1/F2: {error}");
        std::process::exit(1);
    }
    println!("F1/F2 diagnostic active for 30 seconds; press either key (they are temporarily captured globally)");
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE) }
            .as_bool()
        {
            if message.message == WM_HOTKEY {
                match message.wParam.0 as i32 {
                    league_ready_hotkeys::windows::hotkeys::ACCEPT_HOTKEY_ID => {
                        println!("received F1 (accept binding)")
                    }
                    league_ready_hotkeys::windows::hotkeys::DECLINE_HOTKEY_ID => {
                        println!("received F2 (decline binding)")
                    }
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
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, PeekMessageW, RegisterClassW, CS_HREDRAW, CS_VREDRAW,
        HWND_MESSAGE, MSG, PM_REMOVE, WM_APP, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
    };
    let class_name = windows::core::w!("LeagueReadyHotkeysTrayWindow");
    unsafe {
        let _ = RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: Default::default(),
            lpszClassName: class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        });
    }
    let owner = unsafe {
        CreateWindowExW(
            Default::default(),
            class_name,
            windows::core::w!("League Ready Hotkeys"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            None,
            None,
        )
    }
    .unwrap_or_else(|error| {
        eprintln!("could not create tray message window: {error}");
        std::process::exit(1);
    });
    let icon_path = std::path::Path::new("assets\\tray-icon.ico");
    let mut tray = league_ready_hotkeys::windows::tray::TrayIcon::with_icon(owner, icon_path)
        .unwrap_or_else(|error| {
            eprintln!("could not load tray icon: {error}");
            std::process::exit(1);
        });
    if !tray.add() {
        eprintln!("could not add tray icon");
        std::process::exit(1);
    }
    println!("tray icon active for 30 seconds; right-click it to open the menu");
    let deadline = Instant::now() + Duration::from_secs(30);
    'tray: while Instant::now() < deadline {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE) }
            .as_bool()
        {
            if message.message == WM_APP + 2 {
                break 'tray;
            }
            if message.message == WM_APP + 1 {
                println!(
                    "tray callback received: event={} expected={}",
                    message.lParam.0, WM_RBUTTONUP
                );
                if message.lParam.0 as u32 == WM_RBUTTONUP
                    && tray.show_menu(
                        league_ready_hotkeys::windows::startup::is_enabled().unwrap_or(false),
                        league_ready_hotkeys::windows::startup::notifications_enabled()
                            .unwrap_or(true),
                    ) == league_ready_hotkeys::windows::tray::MENU_EXIT
                {
                    break 'tray;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    tray.remove();
    unsafe {
        let _ = DestroyWindow(owner);
    }
    println!("tray icon removed");
}

#[cfg(windows)]
static TRAY_EXIT_REQUESTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
static STARTUP_TOGGLE_REQUESTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
unsafe extern "system" fn tray_wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    if message == league_ready_hotkeys::windows::tray::TRAY_MESSAGE {
        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::WM_APP + 1,
            wparam,
            lparam,
        );
    } else if message == windows::Win32::UI::WindowsAndMessaging::WM_COMMAND {
        match (wparam.0 & 0xffff) as u32 {
            league_ready_hotkeys::windows::tray::MENU_EXIT => {
                TRAY_EXIT_REQUESTED.store(true, std::sync::atomic::Ordering::Release);
            }
            league_ready_hotkeys::windows::tray::MENU_STARTUP => {
                STARTUP_TOGGLE_REQUESTED.store(true, std::sync::atomic::Ordering::Release);
            }
            _ => {}
        }
    }
    windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, message, wparam, lparam)
}

#[cfg(not(windows))]
fn main() {
    eprintln!("league-ready-hotkeys is supported on Windows only");
}
