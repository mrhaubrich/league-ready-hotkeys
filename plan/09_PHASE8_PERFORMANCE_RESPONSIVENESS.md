# Phase 8 — Performance and responsiveness

Objective: remove plausibly user-visible latency and unnecessary continuous background work while preserving the explicit-action safety model, exactly-once action routing, prompt resource cleanup, and credential secrecy.

Entry criteria: LRH-014 is complete with its required real-ready-check evidence or an explicitly accepted reproducible substitute. The Phase 7 automated validation remains green.

Phase status: Active. Entry criteria satisfied on 2026-08-23 by LRH-014 automated evidence and the user's explicit confirmation of its remaining real Windows validation gate.

Exit criteria:

- Tray, notification, shortcut, and shutdown messages remain responsive while LCU discovery or network operations are slow or stalled.
- Accept and Decline remain explicit user actions, are sent at most once, and are never automatically retried.
- A stable League connection reuses cached discovery state and its HTTP client; restart, lockfile change, and request failure invalidate stale state promptly.
- LCU events reduce ready-check activation latency, while reconciliation polling remains available for initial state, missed events, and reconnect recovery.
- The idle parent no longer wakes on a fixed 25 ms cadence solely to poll messages or hook actions.
- Low-level hook callbacks avoid allocations, blocking logging, and unnecessary locks, and only required hook types are installed.
- Tray-menu and notification-child work cannot starve ready-check monitoring or shutdown.
- Diagnostics terminate at their documented bounds, and JSON/dependency changes are made only when measurement justifies them.
- Windows ETW/WPR, Process Monitor, handle-count, and real-ready-check evidence record latency, wakeups, cleanup, reconnect, and exactly-once behavior.

## LRH-015 — Decouple LCU I/O from the Win32 message loop

Priority: MUST. Status: Validation-required. Dependencies: LRH-014. RICE: 5 × 3 × 0.8 / 2 = 6.0.

Move discovery and LCU HTTP work to a dedicated worker/runtime thread, report state and completions to the owner window, and configure measured loopback connection/read/total timeouts. Claim the existing action gate synchronously before dispatching an explicit action.

Acceptance:

- A stalled GET or POST cannot block tray, notification, shortcut, or shutdown message handling.
- Accept and Decline remain user-initiated and at most one action is in flight per ready check.
- Action POSTs are never automatically retried after timeout or ambiguous transport failure.
- Disconnect and shutdown cancel or bound outstanding work and promptly release hotkeys, hooks, tray state, and notification resources.
- Errors and diagnostics never include lockfile credentials or authenticated URLs.

Validation: add deterministic stalled-transport tests and a credential-free hanging-loopback diagnostic; measure tray/Exit latency with WPR while the transport stalls; then perform a safe real-ready-check Accept/Decline exactly-once scenario.

Implementation evidence (2026-08-23): production discovery, lockfile reading/parsing, HTTP client construction, ready-check GETs, and explicit Accept/Decline POSTs now run on one named LCU worker with a worker-owned current-thread Tokio runtime. The Win32 owner submits bounded commands and receives state/action completions through a channel plus a custom posted window message; the background message loop contains no LCU `block_on`, discovery, lockfile read, client construction, or HTTP request. Only one poll may be outstanding. A shared `ActionGate` claims every hotkey, low-level-hook, or notification action synchronously before enqueueing; matching ready-check generations suppress stale completions and resource reactivation. Action failure schedules fresh state reconciliation but never retries the POST. Shutdown disables hotkeys/hooks/notification, removes the tray icon, marks the worker stopping, skips queued actions, and joins only the currently bounded operation.

Transport evidence: reqwest now uses a 250 ms loopback connection timeout and a 1.5 second read/total request timeout. Errors crossing the worker boundary are reduced to readiness or success state and never carry credentials, authenticated URLs, or raw lockfile data. Worker/channel failure is treated as disconnect and disables active resources.

Executable validation: deterministic tests prove that a stalled transport does not block the command caller, a failed explicit action is attempted exactly once without retry, and shutdown skips an action queued behind stalled work. Rustfmt passed; all 43 tests passed; Clippy passed with warnings denied; an isolated optimized release build passed without replacing the user's running utility. The credential-free hanging-TLS-loopback diagnostic passed with 9 microseconds submission latency, 252 milliseconds bounded stalled response, and 420167 caller progress iterations. `git diff --check` passed.

Validation evidence (2026-08-23): the user rebuilt/restarted the normal release utility, exercised the requested real Windows responsiveness and ready-check scenario, and explicitly confirmed that it works greatly. This closes the tray/Exit responsiveness, bounded transport, exactly-once action, outside-ready-check, disconnect, and shutdown cleanup gate. LRH-015 is complete.

## LRH-016 — Cache League discovery state and reuse the HTTP client

Priority: MUST. Status: Complete. Dependencies: LRH-015. RICE: 3 × 2 × 1.0 / 1 = 6.0.

Cache the discovered process/lockfile identity, parsed connection parameters, and `LcuClient`. Reuse the connection pool while the client remains valid; invalidate on process exit, lockfile metadata change, authentication/connection failure, or League restart.

Acceptance:

- Stable idle operation does not refresh the full process table, reread/reparse the lockfile, or rebuild the HTTP client every two seconds.
- League restart or port/credential change invalidates stale state and reconnects using bounded backoff.
- Replaced credentials are dropped and never logged, persisted, or placed in diagnostic output.
- Cache invalidation cannot leave hooks or notifications active after disconnect.

Validation: use Process Monitor and WPR/WPA to compare ten-minute process-query, lockfile-read, allocation, connection, and TLS counts; restart League and verify recovery plus cleanup.

Implementation evidence (2026-08-23): the LCU worker now retains a client cache keyed by lockfile path, length, and successful last-modified metadata. Stable polls reuse the parsed credentials and reqwest connection pool; cache misses alone perform process discovery, lockfile read/parse, and HTTP client construction. A metadata/path change rebuilds the client and drops the old credential-bearing object. Any ready-check or action transport error invalidates the cache, so League restarts and stale ports cannot retain active state indefinitely. Metadata inspection failure is treated as invalidation rather than an unchanged file.

Executable validation: the cache identity test covers same-path/same-metadata reuse and path/metadata invalidation. Rustfmt passed; all 44 tests passed; Clippy passed with warnings denied; an isolated optimized release build passed. The credential-safe `target\lrh019\release\league-ready-hotkeys.exe --check-lcu-cache` diagnostic passed with `client-builds=1 cache-reuses=1` while League was running. No lockfile content, password, or authenticated URL is printed.

Validation evidence (2026-08-23): automated tests, warnings-denied Clippy, isolated release build, and the credential-safe cache diagnostic passed. The user confirmed the cache/restart validation scenario works. LRH-016 is complete.

## LRH-017 — Combine ready-check events with reconciliation polling

Priority: SHOULD. Status: Planned. Dependencies: LRH-015, LRH-016. RICE: 5 × 2 × 0.8 / 2 = 4.0.

Run the existing LCU WebSocket subscription on the worker for low-latency transitions. Keep a slower GET reconciliation path for initial state, missed events, connection loss, and event/API drift.

Acceptance:

- Ready-check events activate state without waiting for the polling interval.
- Initial connection, a deliberately dropped socket, and missed/duplicate events converge through polling without duplicate resource registration.
- Reconnect/backoff does not spin, auto-act, or retain stale active state.
- Event payloads and errors remain credential-safe.

Validation: timestamp event receipt through hook/notification activation with `QueryPerformanceCounter`; kill the socket, inject duplicate events, and verify poll recovery and exactly-once state transitions before a real ready check.

## LRH-018 — Replace the fixed 25 ms wake loop with message-driven waiting

Priority: SHOULD. Status: Locked. Dependencies: LRH-015, LRH-017. RICE: 3 × 2 × 0.8 / 1.5 = 3.2.

Replace unconditional `PeekMessageW` plus 25 ms sleep with a message/event wait whose timeout is the next scheduled reconciliation. Have hook matches post an owner-window message rather than depend on periodic atomic polling.

Acceptance:

- The idle process does not wake approximately 40 times per second solely for loop maintenance.
- Tray, shortcut, notification, and shutdown messages wake the loop promptly.
- There is no lost-wakeup race at the transition into a wait.
- Action gating and cleanup behavior remain unchanged.

Validation: compare idle wakeups, context switches, CPU, input-to-dispatch latency, and Exit latency with WPR; stress actions at wait boundaries and verify no loss or duplication.

## LRH-019 — Make low-level hook callbacks bounded and allocation-free

Priority: MUST. Status: Complete. Dependencies: LRH-014. RICE: 5 × 3 × 1.0 / 1.5 = 10.0.

Precompile bindings into virtual-key/button values and modifier bitmasks, remove production console writes from hook callbacks, avoid per-event allocation and string normalization, minimize synchronization, and install only the keyboard/mouse hook types required by active bindings.

Acceptance:

- Keyboard and mouse callbacks perform no heap allocation or blocking output.
- Nonmatching input returns immediately through `CallNextHookEx` and is never suppressed.
- Exact modifier, mouse-button, collision, active-ready-check, and action-routing semantics remain unchanged.
- Hooks still disappear after action, ready-check end, disconnect, and shutdown.

Validation: add binding-equivalence and lifecycle tests; record callback-duration histograms without logging inside callbacks; stress high-rate input and repeat the real-ready-check exactly-once/outside-check/cleanup scenario.

Implementation evidence (2026-08-23): configured bindings are compiled once into packed integer key/button and modifier representations before hook installation. Callback matching now uses atomic binding snapshots, integer comparisons, and an atomic first-action claim; it performs no callback-path heap allocation, mutex acquisition, string normalization, formatting, or console output. Unrelated keyboard events are rejected before modifier-state queries. Installation selects only the keyboard and/or mouse hook types required by the configured pair, cleanly replaces an existing installation, rolls back partial failure, and clears pending actions and bindings before unhooking. Callback timing is opt-in for the diagnostic and records six atomic histogram buckets without logging inside callbacks.

Executable validation: four binding/device/timing tests plus the fast-rejection test cover legacy matching equivalence, keyboard-only, mouse-only, mixed-device selection, timing reset, and unrelated-key behavior. The credential-free shortcut diagnostic now installs the configured device hooks, replaces them once, verifies the observed hook types, and proves final cleanup. Automated evidence: Rustfmt passed; all 40 tests passed; Clippy passed with warnings denied; an isolated optimized release build passed without interrupting the user's running utility; `target\lrh019\release\league-ready-hotkeys.exe --check-shortcuts` printed `configured hooks keyboard=true mouse=false replaced and released` for the current F1/F2 configuration. Source inspection confirms neither low-level callback contains `Vec`, string conversion, mutex locking, or logging.

Validation evidence (2026-08-23): after the automated evidence above, the user ran the requested real Windows sustained-input and ready-check validation and explicitly confirmed that everything worked. This closes the callback responsiveness, explicit Accept/Decline, outside-ready-check inactivity, and cleanup regression gate. LRH-019 is complete.

## LRH-020 — Keep ready-check monitoring alive during tray menus

Priority: MUST. Status: Locked. Dependencies: LRH-015, LRH-017. RICE: 5 × 3 × 0.8 / 1 = 12.0.

Ensure the synchronous `TrackPopupMenu` interaction cannot pause LCU discovery, event monitoring, reconciliation, or cleanup. Reuse the worker/message architecture rather than replacing the native tray menu.

Acceptance:

- Holding the tray menu open longer than a ready check does not stop detection or leave stale active resources.
- Menu selection, dismissal, and Exit remain correct after queued state changes.
- No ready-check transition causes an automatic action or duplicate registration.

Validation: hold the menu open for at least 15 seconds across ready-check activation, cancellation, and League disconnect; verify state convergence, one notification/hook lifecycle, and prompt Exit.

## LRH-021 — Move notification child process I/O and teardown off the message thread

Priority: SHOULD. Status: Planned. Dependencies: LRH-014, LRH-015. RICE: 3 × 2 × 0.8 / 1.5 = 3.2.

Move notification child spawn, pipe serialization/write/flush, reap, and forced termination to a bounded controller worker. Associate commands and child responses with a ready-check generation so stale renderer output cannot authorize an action.

Acceptance:

- A slow spawn, blocked pipe, or child ignoring `Close` cannot block tray or shutdown message handling.
- The existing sub-one-second child cleanup bound is preserved or improved.
- Late child messages after inactive/disconnect/action/shutdown are ignored.
- The child still receives no LCU credentials and can never contact LCU directly.

Validation: use a test child that stops reading and ignores `Close`; measure UI/Exit latency, forced cleanup, rapid reopen/close cycles, stale-response rejection, and exactly-once actions.

## LRH-022 — Bound the low-level hook diagnostic runtime

Priority: COULD. Status: Planned. Dependencies: LRH-019. RICE: 1 × 1 × 1.0 / 0.5 = 2.0.

Replace the diagnostic's indefinitely blocking `GetMessageW` wait with a timer or timed message wait that observes the documented deadline without user input.

Acceptance:

- The diagnostic exits at its documented bound when no input arrives.
- Keyboard and mouse hooks are removed on success, timeout, error, and shutdown paths.
- Diagnostic timer/message identifiers cannot affect production behavior.

Validation: run the diagnostic without input past its deadline, repeat with keyboard/mouse input, and inspect hook/process cleanup.

## LRH-023 — Remove unnecessary ready-check JSON copies

Priority: COULD. Status: Locked. Dependencies: LRH-016, LRH-017. RICE: 1 × 0.5 × 0.8 / 0.5 = 0.8.

Deserialize HTTP responses directly into the typed ready-check model and use a minimal typed WebSocket envelope, provided benchmarks show a measurable allocation or CPU reduction.

Acceptance:

- The full `serde_json::Value` tree is not cloned during normal ready-check parsing.
- Missing/additional-field tolerance and current error classification remain compatible with sanitized contract fixtures.
- No raw credential-bearing transport data is logged.

Validation: benchmark sanitized HTTP/event payloads and allocation counts before implementation; retain the existing implementation if the result is immaterial; rerun contract and reconnect tests after any change.

## LRH-024 — Audit dependency and release footprint using measurements

Priority: COULD. Status: Planned. Dependencies: LRH-014. RICE: 1 × 0.5 × 1.0 / 0.5 = 1.0.

Measure crate contribution, cold start, private working set, and binary size. Remove only proven-unused direct dependencies and Windows feature groups; consolidate TLS implementations only if measurements show a material benefit without weakening LCU compatibility.

Acceptance:

- `cargo bloat` or equivalent evidence identifies each candidate before removal.
- Release build, diagnostics, self-signed localhost TLS, WebSocket reconnect, notification rendering, and credential redaction remain valid.
- No framework or transport rewrite is introduced solely for binary size.

Validation: record release executable size, cold-start trace, idle private working set, and crate contribution before/after each isolated change; retain only improvements with material evidence.

## Phase risks and decisions

- Responsiveness work must not weaken the explicit-action gate: no timer, event, reconnect, timeout, or retry may initiate Accept or Decline.
- Ambiguous POST failures are terminal for that explicit attempt; automatic retries could duplicate an action.
- Events complement polling because either mechanism alone can miss state during connection and contract failures.
- Timeout values require Windows loopback measurements; values that are too short could reject healthy League responses.
- Worker messages and notification IPC require generation/state checks so delayed work cannot revive stale ready checks.
- Hook optimization must preserve exact matching and pass-through behavior before allocation or lock reductions are accepted.
- Dependency and JSON work is measurement-gated and must stop if the benefit is immaterial.
