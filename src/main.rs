#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str);
    if mode == Some("--check-hotkeys") {
        run_hotkey_diagnostic();
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

#[cfg(not(windows))]
fn main() {
    eprintln!("league-ready-hotkeys is supported on Windows only");
}
