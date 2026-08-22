# Phase 5 — Background mode and ready-check UI

Objective: make the release executable useful when launched normally and provide a small, non-intrusive bottom-right prompt whenever a ready check is active.

Entry criteria: Phase 4 release validation is complete.

Exit criteria:

- Launching without arguments starts the background utility and tray lifecycle.
- League discovery, reconnect, ready-check state, and conditional hotkeys run together without blocking the Windows message loop.
- An active ready check displays a bottom-right notification stating that F1 accepts and F2 declines.
- The notification has explicit Accept and Decline buttons that use the same guarded commands as F1/F2.
- The notification disappears after a successful action, ready-check cancellation, League disconnect, or timeout.
- No notification or hotkey captures occur while no ready check is active.
- UI, reconnect, and idle-resource behavior have executable diagnostics and manual Windows evidence.

## Tasks

### LRH-009 — Add default always-running background mode

Priority: MUST. Dependencies: LRH-008. Status: Planned.

Implement the production application orchestration for no-argument launch: discovery worker, LCU watcher/reconnect loop, application state messages, Windows message loop, tray icon, conditional F1/F2 registration, clean shutdown, and single-instance protection if required by the chosen UI approach.

Acceptance: no-argument launch remains alive with League absent, discovers a running client, transitions into and out of ready-check state, releases hotkeys on disconnect, and exits from the tray without leaving an icon.

Validation: add a `--diagnose-background` harness or equivalent deterministic fake-transport test plus manual launch/restart/League-shutdown instructions.

### LRH-010 — Add bottom-right ready-check notification window

Priority: MUST. Dependencies: LRH-009. Status: Planned.

Implement a native Windows notification window positioned above the taskbar work area in the bottom-right corner. It must be owned by the app, visually compact, keyboard-accessible, DPI-aware, and shown only for an active ready check. The text must clearly say that F1 accepts and F2 declines.

Acceptance: the prompt appears on activation, remains above ordinary windows without stealing focus, handles monitor/taskbar positioning, and closes on inactive/disconnect/action/timeout transitions.

Validation: add a `--check-notification` diagnostic and manual multi-monitor, taskbar, focus, and DPI scenarios.

### LRH-011 — Add notification Accept/Decline buttons

Priority: MUST. Dependencies: LRH-010. Status: Locked.

Wire buttons to the existing action gate and LCU endpoints. Revalidate ready-check state immediately before sending, disable both buttons while a request is in flight, and display a short success/error state without exposing credentials.

Acceptance: clicking Accept calls only the accept endpoint, clicking Decline calls only the decline endpoint, duplicate clicks are suppressed, and stale prompts cannot send actions.

Validation: add deterministic command-routing tests and a live `--check-action`-equivalent UI test procedure using safe test queues.

### LRH-012 — Add UI/tray status and notification preferences

Priority: SHOULD. Dependencies: LRH-010. Status: Locked.

Add tray status text and minimal user preferences: notification enabled/disabled, display duration, and optional launch-at-startup integration. Keep v1 configuration local and secret-free; do not add automatic acceptance or broader LCU automation.

Acceptance: preferences persist per user, take effect without restart where practical, and are reflected in the tray menu.

Validation: add registry/config tests and a manual toggle/restart scenario.

## Risks and decisions

- Prefer a native Win32 notification window to keep idle overhead low; avoid introducing a heavyweight embedded browser runtime.
- Do not use toast registration or external notification services unless a later architecture decision accepts their packaging and permission costs.
- The prompt must never steal focus or intercept keys outside an active ready check.
- Button actions share the same state gate and endpoint safeguards as global hotkeys.
- Multi-monitor work-area calculation and DPI scaling require manual Windows validation.
