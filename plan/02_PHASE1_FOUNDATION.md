# Phase 1 — Foundation

Objective: establish a testable Windows Rust project and safe LCU discovery boundary.

Entry: approved plan and Rust toolchain available. Exit: project builds, lockfile parsing is tested, and authenticated read-only LCU validation is recorded. LRH-001 and LRH-002 are complete. Evidence: `target/release/league-ready-hotkeys.exe --check-lockfile` discovered LeagueClientUx.exe and returned `LCU reachable; no active ready check (HTTP 404)`. Exclusions: hotkeys, tray, automatic actions. Gate: tests and Clippy passed; formatting remains a pre-existing workspace permission issue.
