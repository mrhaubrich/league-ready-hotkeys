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

Priority: SHOULD. Status: Planned. Dependencies: LRH-010, LRH-011, LRH-012, LRH-013.

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

Risks:

- IPC failure could leave a stale child or lose an action; parent state remains authoritative and must time out/terminate safely.
- A child-process UI must not inherit or receive LCU lockfile credentials.
- Window activation flags must be validated on Windows because framework defaults may steal focus.
- Renderer startup latency must not materially delay a short ready check.
