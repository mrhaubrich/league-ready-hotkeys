# League Ready Hotkeys

A small Windows tray utility for explicit League Client ready-check actions:

- F1 accepts the active ready check.
- F2 declines the active ready check.
- Keys are captured only while a ready check is active.

The League Client API is a local, unsupported interface. The application must never log or persist the lockfile password, and it must never auto-accept a match.

## Development

Requires the stable Rust MSVC toolchain. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`.

The current foundation parses an LCU lockfile and provides the Windows-only authenticated transport boundary. No credentials are written to disk or included in diagnostics.

For a real read-only connection check, run `league-ready-hotkeys.exe --check-lockfile`. It discovers the running `LeagueClientUx.exe` and adjacent lockfile automatically. An optional lockfile path can still be supplied. A `404` is expected when the client is not attached to a matchmaking queue; it confirms the LCU is reachable.

To validate live ready-check events, run `league-ready-hotkeys.exe --watch-ready-check` before queueing. It waits for the LCU event and prints the sanitized active/response state when a ready check appears.

To validate global key registration independently, run `league-ready-hotkeys.exe --check-hotkeys`, press F1 and F2 during the 30-second window, and confirm it prints the corresponding binding. The command temporarily captures both keys globally and releases them when it exits.

To validate endpoint routing explicitly, run `league-ready-hotkeys.exe --check-action accept` or `league-ready-hotkeys.exe --check-action decline` while a ready check is visible. The command refuses to send anything unless the live ready-check payload is active; each invocation is an explicit user action.

Startup registration can be validated with `--enable-startup`, `--check-startup`, and `--disable-startup`. These commands use only the current user's `HKCU` Run key and require no administrator rights.

Tray lifecycle can be validated with `--check-tray`; it adds the native tray icon for 30 seconds and removes it on exit.

Notification rendering can be validated without entering a queue with `--check-notification`; it displays the bottom-right prompt for 30 seconds using the persisted Accept and Decline bindings. Run `league-ready-hotkeys.exe --set-bindings Ctrl+Shift+P Mouse4`, then `league-ready-hotkeys.exe --check-notification`, and confirm the Accept button shows three keycaps (`Ctrl`, `Shift`, `P`) while Decline shows an `MB4` keycap.

The notification countdown uses the LCU ready-check `timer` field. Live contract validation on 2026-08-22 confirmed that the current client reports elapsed seconds from 0 through 12 before closing the endpoint; the UI converts that authoritative value into remaining seconds and a shrinking progress bar.

Shortcut configuration can be inspected with `--check-shortcuts` and changed with `--set-shortcuts F3 F4`. F1/F2 remain the defaults until another pair is saved; duplicate or unsupported keys are rejected.

To configure shortcuts from the app, right-click the tray icon and choose `Configure hotkeys...`. Select `Accept` or `Decline`, then press the desired keyboard combination or mouse button. The pair is validated, saved per user, and reflected by the next notification without restarting the app.

The Slint Fluent-dark settings window can be validated independently with `league-ready-hotkeys.exe --check-settings`; it opens for 60 seconds with native Windows drag, minimize, maximize, and close controls. Changes remain staged until `Save changes`; `Cancel` reloads the persisted bindings.

Arbitrary binding syntax can be validated with `--check-binding Ctrl+Shift+A`, `--check-binding Alt+R`, or `--check-binding Mouse4`.

Low-level keyboard and mouse input hooks can be validated with `--check-input-hook`; press keys and mouse buttons during its 30-second diagnostic window.

Launching the executable without arguments now starts the background utility. It discovers League every two seconds, polls the ready-check state, and registers F1/F2 only while an unanswered ready check is active. Use the tray menu to exit.

## Release validation

From PowerShell, run `.scripts\validate-release.ps1`. It runs formatting, Clippy, all tests, and the optimized release build, then prints the portable executable size and SHA-256 checksum. Copy `target\release\league-ready-hotkeys.exe` together with that checksum; no installer or registry export is required.

GitHub Actions builds the Windows x64 release on every push and pull request. The workflow publishes an executable artifact and a portable bundle containing the executable and tray icon asset. Until the tray icon is embedded as a Windows resource, use the portable bundle for a complete installation.
