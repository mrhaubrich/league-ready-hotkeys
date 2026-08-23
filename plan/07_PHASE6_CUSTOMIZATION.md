# Phase 6 — Shortcut customization

Objective: allow users to choose arbitrary global keyboard or mouse shortcuts used to accept and decline a ready check while preserving safe registration and the F1/F2 defaults.

Entry criteria: Phase 5 is complete.

Exit criteria:

- F1 and F2 remain the default bindings for existing users.
- Users can configure separate Accept and Decline shortcuts from a clearly reachable tray/settings UI.
- Shortcuts may contain keyboard modifiers (`Ctrl`, `Alt`, `Shift`, `Win`), letters, numbers, function keys, navigation keys, punctuation, and mouse buttons.
- Choices persist per user without storing credentials.
- Invalid, duplicate, reserved, or unavailable shortcuts are rejected with a clear explanation.
- Shortcuts are registered only during an active ready check and are released on every inactive/disconnect/shutdown path.
- Changing shortcuts while active cannot leave stale registrations behind.

## LRH-013 — Add configurable accept/decline shortcuts

Priority: SHOULD. Status: Complete. Dependencies: LRH-009, LRH-012.

Add a shortcut configuration model and registry-backed persistence. Extend the tray UI with a capture flow that records the next keyboard or mouse input. Use `RegisterHotKey` where possible and narrowly scoped low-level keyboard/mouse hooks for combinations and mouse buttons that cannot be represented by `RegisterHotKey`. Validate modifier/key combinations, prevent Accept and Decline collisions, unregister old bindings before applying new ones, and fall back safely to F1/F2 if stored settings are malformed.

Acceptance:

- A fresh install uses F1/F2.
- A configured pair survives restart.
- A failed registration leaves the previous working pair intact.
- The chosen shortcuts trigger only their respective LCU actions during an active ready check.
- No shortcut captures keys outside an active ready check.
- Mouse and keyboard hooks are installed only while needed and are removed on inactive state, disconnect, and shutdown.

Validation:

- Unit-test serialization, defaults, malformed values, duplicate detection, and conflict rollback.
- Add a diagnostic command such as `--check-shortcuts` that prints sanitized configured bindings and exercises registration/unregistration.
- Manually configure, restart, enter a safe ready check, and validate both actions.

Implementation evidence (2026-08-23): the ready-check notification reads persisted arbitrary bindings and renders keyboard combinations as individual keycaps and mouse bindings as matching `MB4`/`MB5` keycaps. The tray exposes `Configure hotkeys...`; its settings surface is now a Slint Fluent-dark window on a dedicated UI event loop, with full clickable action cards, keyboard/mouse capture, collision validation, staged Save/Cancel behavior, registry persistence, tray reopen after close, and live notification refresh. `--check-shortcuts` is a credential-free diagnostic: it validates the saved non-colliding pair, confirms Windows-logo-key bindings are rejected as reserved, temporarily registers and releases F1/F2 plus the configured low-level hooks, and never contacts League. The registration seam has deterministic automated rollback coverage: a failed second registration unregisters the first. Automated checks passed: Rustfmt, 35 tests, Clippy with warnings denied, debug build, `git diff --check`, and `cargo run -- --check-shortcuts` printed `shortcut diagnostic passed: accept=F1 decline=F2; reserved Win bindings rejected; temporary registrations and configured hooks released`. Real-client evidence: the user configured and restarted the release application, entered safe ready checks, and confirmed that configured Accept and Decline each sent exactly one action, shortcuts did nothing outside a ready check, and hooks disappeared after an action, League disconnect, and tray shutdown. LRH-013 is complete.

Risks:

- Global shortcut conflicts with other applications.
- Low-level hooks can affect input latency or interfere with games if their lifetime is mishandled.
- Some keys have special Windows or game behavior.
- A portable executable moved after configuration must retain settings without exposing secrets.
