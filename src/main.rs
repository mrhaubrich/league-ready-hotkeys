#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str);
    if !matches!(mode, Some("--check-lockfile") | Some("--watch-ready-check")) {
        println!("use --check-lockfile or --watch-ready-check");
        return;
    }
    let path = args.get(2).cloned().or_else(|| league_ready_hotkeys::lcu::discover_lockfile().ok().map(|path| path.to_string_lossy().into_owned()));
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
        client.ready_check().await.map_err(|error| error.to_string())
    });
    match result {
        Ok(Some(payload)) => println!("LCU reachable; ready-check response: {payload}"),
        Ok(None) => println!("LCU reachable; no active ready check (HTTP 404)"),
        Err(error) => { eprintln!("LCU request failed: {error}"); std::process::exit(1); }
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("league-ready-hotkeys is supported on Windows only");
}
