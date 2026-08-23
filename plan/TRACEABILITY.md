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
| Shortcut customization | 5 | LRH-013 | persisted binding display implemented in `src/windows/notification.rs`; `Ctrl+Shift+A` and `Mouse4`-to-`MB4` keycap tests pass; local diagnostic rendered configured `Ctrl+Shift+P`/`Mouse4`; live ready-check action validation remains |
