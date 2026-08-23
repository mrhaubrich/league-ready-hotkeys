# Phase 7 — Slint ready-check notification

Objective: replace the hand-painted Win32/GDI ready-check notification with a polished Slint Fluent-dark surface without changing LCU, hotkey, or action safety behavior.

Entry criteria: LRH-010 through LRH-013 are implemented and their automated validation remains green.

Exit criteria:

- The notification matches the established League-inspired visual hierarchy and renders configured keyboard and mouse bindings as centered keycaps.
- Accept remains visually dominant; Decline remains quieter and outlined; both cards are fully clickable.
- The notification is topmost, bottom-right positioned, and does not steal focus.
- Countdown progress uses only the observed LCU elapsed timer contract and never implies an unsupported duration.
- Slint runs in a short-lived child process that exists only while a ready check is active, preserving the lightweight parent process at idle.
- Closing, ready-check completion, disconnect, and shutdown terminate the notification child and release its UI resources.
- Accept and Decline are explicit user actions sent to the parent over bounded IPC; the child never calls LCU or auto-accepts.

## LRH-014 — Migrate ready-check notification to Slint

Priority: SHOULD. Status: Complete. Dependencies: LRH-010, LRH-011, LRH-012, LRH-013.

Implement a Slint notification child mode and a parent-owned controller. Use a narrow IPC contract for visibility/lifecycle, observed timer updates, configured binding labels, and Accept/Decline action messages. Keep LCU transport and action authorization exclusively in the parent process.

Acceptance:

- Activating a ready check launches one notification child; repeated active events do not launch duplicates.
- Timer and binding updates appear without recreating the window.
- Clicking Accept or Decline produces exactly one matching parent action while the ready check is active.
- Inactive, disconnect, shutdown, and completed actions terminate the child within one second.
- Repeated ready checks can open and close the notification without Slint platform reinitialization errors.
- Parent idle working set does not retain the Slint renderer after the child exits.

Validation:

- Unit-test IPC command parsing, binding/keycap formatting, timer clamping, duplicate-action suppression, and child lifecycle state.
- Add a `--check-slint-notification` diagnostic that exercises launch, live timer/binding updates, both click responses, close, and reopen without LCU credentials.
- Run Rustfmt, the full test suite, Clippy with warnings denied, and release build.
- User-run scenario: start the release app, enter two safe ready checks, exercise Accept and Decline once each, verify focus remains in the foreground app, then confirm the notification child exits after each check.

Implementation evidence (2026-08-23): the Win32/GDI notification was replaced by a Slint Fluent-dark child process and parent-owned controller. Line-delimited JSON over piped stdin/stdout carries only clamped observed timer values, formatted binding keycaps, lifecycle commands, and explicit action messages; the child has no LCU client or lockfile access. Winit window attributes request bottom-right work-area positioning, always-on-top presentation, taskbar omission, and `active(false)` before window creation; the first guaranteed native-window event adds Win32 `WS_EX_NOACTIVATE` before reporting the child ready. The parent suppresses duplicate child launches and actions, terminates the child after actions/inactive/disconnect/shutdown with a 900 ms graceful bound followed by forced cleanup, and keeps Slint renderer initialization in the short-lived child. Automated evidence: Rustfmt, 35 tests, Clippy with warnings denied, and optimized release build passed. `target\release\league-ready-hotkeys.exe --check-slint-notification` passed timer/binding updates, duplicate Accept suppression, clean close, child reopen, duplicate Decline suppression, and second clean close without LCU credentials. Windows UI automation could locate the topmost `Ready check` window but could not foreground/capture it because the non-activating window reported no foreground process ID. The user explicitly confirmed the remaining real Windows validation gate on 2026-08-23 and authorized recording LRH-014 as validated. LRH-014 is complete.

Risks:

- IPC failure could leave a stale child or lose an action; parent state remains authoritative and must time out/terminate safely.
- A child-process UI must not inherit or receive LCU lockfile credentials.
- Window activation flags must be validated on Windows because framework defaults may steal focus.
- Renderer startup latency must not materially delay a short ready check.
