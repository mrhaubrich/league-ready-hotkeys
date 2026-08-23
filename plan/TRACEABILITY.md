# Traceability

| Goal | Phase | Tasks | Code/tests/evidence |
|---|---|---|---|
| F1 accepts and F2 declines | 2 | LRH-003–005 | `src/app.rs`, `src/lcu`, `src/windows`, ready-check integration tests |
| Secure LCU session boundary | 1 | LRH-002 | `src/lcu/discovery.rs`, `src/lcu/transport.rs`, parser tests; live release diagnostic returned HTTP 404 from reachable LCU |
| Ready-check activation state | 2 | LRH-003 | `src/lcu/ready_check.rs`, WebSocket watcher, sanitized event tests; live watcher printed `active:true,response:None` |
| Conditional global hotkeys | 2 | LRH-004 | `src/windows/hotkeys.rs`, `--check-hotkeys`; live F1/F2 messages received and Zed F1 suppressed |
| Tray icon lifecycle | 3 | LRH-006 | `src/windows/tray.rs`, `--check-tray`; right-click callback received event 517, menu closed the app, and icon was removed |
| Safe accept/decline action routing | 2 | LRH-005 | `src/lcu/transport.rs`, `src/app.rs::ActionGate`, 12 tests; live `--check-action decline` declined a real match |
| No accidental in-game capture | 2/3 | LRH-004, LRH-007 | hotkey lifecycle tests and manual Windows scenario |
| Starts with Windows | 3 | LRH-006 | `src/windows/startup.rs`, registry manual check |
| Low idle usage | 4 | LRH-007/008 | reconnect tests and measured release footprint |
| Reconnect and shutdown | 3 | LRH-007 | `--watch-ready-check` connected to the live client; `--check-tray` removed the icon and exited cleanly |
| Portable release | 4 | LRH-008 | release validation script passed; tray runtime and idle CPU/working-set measurements were acceptable |
| Normal launch and ready-check prompt | 5 | LRH-009–LRH-010 | native League-inspired notification and LCU-backed 12-second countdown implemented; current `/help` contract and sanitized live timer range validated; multi-monitor/focus/DPI user evidence remains |
| Notification actions | 5 | LRH-011 | live background validation succeeded: clicking Decline printed the diagnostic and declined the real ready check |
| Notification preference persistence | 3 | LRH-012 | user confirmed the Notifications checkmark and persistence after restart |
| Shortcut customization | 5 | LRH-013 | tray `Configure hotkeys...` opens the Slint Fluent-dark capture UI in `src/windows/settings.rs`; dedicated UI event loop, staged Save/Cancel, collision-checked registry persistence, close/reopen lifecycle, and live notification refresh implemented; `--check-shortcuts` passed with default F1/F2, reserved Windows-logo-key rejection, temporary registration cleanup, and configured hook cleanup; deterministic rollback test covers a failed second registration; user confirmed configured Accept and Decline each acted exactly once in real ready checks, did nothing outside a ready check, and hooks disappeared after action, disconnect, and shutdown |
| Slint ready-check notification | 5 | LRH-014 | Slint Fluent-dark child plus parent controller implemented with bounded JSON IPC, clamped LCU timer updates, centered binding keycaps, duplicate launch/action suppression, pre-creation inactive positioning plus Win32 `WS_EX_NOACTIVATE`, and sub-one-second cleanup; 35 tests and release checks pass; `--check-slint-notification` passed update/Accept/close/reopen/Decline/close without credentials; user explicitly confirmed the remaining real Windows validation gate on 2026-08-23; LRH-014 complete |
| Responsive Windows message loop | 8 | LRH-015, LRH-018, LRH-020–021 | Planned: worker-owned LCU and notification blocking work, bounded transport/child waits, message-driven wakeups, tray-menu starvation scenario, and WPR latency evidence |
| Efficient League discovery and transport reuse | 8 | LRH-016 | Planned: cached process/lockfile/client state with failure/restart invalidation; Process Monitor and WPR before/after evidence |
| Low-latency resilient ready-check state | 8 | LRH-017 | Planned: WebSocket events plus reconciliation polling, duplicate/missed-event tests, reconnect recovery, and event-to-activation timing |
| Bounded low-level input hooks | 8 | LRH-019, LRH-022 | Planned: allocation-free silent callbacks, device-specific installation, binding-equivalence tests, callback timing, deadline-bound diagnostic, and cleanup evidence |
| Allocation and release-footprint evidence | 8 | LRH-023–024 | Planned: sanitized payload allocation benchmarks, crate-size audit, cold-start/private-working-set measurements, and changes only for material improvements |
