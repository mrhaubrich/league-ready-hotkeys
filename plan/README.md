# Delivery plan

Active phase: Phase 8 — Performance and responsiveness. Phase 7 and LRH-014 are complete.

Selection rule: choose the highest-RICE unblocked task in the active phase; dependencies, security gates, and human decisions always take precedence.

| ID | Task | Priority | Status | Dependencies |
|---|---|---|---|---|
| LRH-001 | Bootstrap repository and Rust application | MUST | Complete | — |
| LRH-002 | Discover and authenticate to LCU securely | MUST | Complete | LRH-001 |
| LRH-003 | Consume and model ready-check state | MUST | Complete | LRH-002 |
| LRH-004 | Implement conditional Windows hotkeys | MUST | Complete | LRH-001, LRH-003 |
| LRH-005 | Implement accept/decline commands | MUST | Complete | LRH-002–004 |
| LRH-006 | Add tray lifecycle and startup toggle | SHOULD | Complete | LRH-005 |
| LRH-007 | Harden reconnect, shutdown, and secrets | MUST | Complete | LRH-006 |
| LRH-008 | Validate and package portable release | MUST | Complete | LRH-007 |
| LRH-009 | Add default always-running background mode | MUST | Complete | LRH-008 |
| LRH-010 | Add bottom-right ready-check notification window | MUST | Planned | LRH-009 |
| LRH-011 | Add notification Accept/Decline buttons | MUST | Complete | LRH-010 |
| LRH-012 | Add UI/tray status and notification preferences | SHOULD | Complete | LRH-010 |
| LRH-013 | Add fully customizable keyboard/mouse shortcuts | SHOULD | Complete | LRH-009, LRH-012 |
| LRH-014 | Migrate ready-check notification to Slint | SHOULD | Complete | LRH-010–013 |
| LRH-015 | Decouple LCU I/O from the Win32 message loop | MUST | Planned | LRH-014 |
| LRH-016 | Cache League discovery state and reuse the HTTP client | MUST | Locked | LRH-015 |
| LRH-017 | Combine ready-check events with reconciliation polling | SHOULD | Locked | LRH-015–016 |
| LRH-018 | Replace the fixed 25 ms wake loop with message-driven waiting | SHOULD | Locked | LRH-015, LRH-017 |
| LRH-019 | Make low-level hook callbacks bounded and allocation-free | MUST | Validation-required | LRH-014 |
| LRH-020 | Keep ready-check monitoring alive during tray menus | MUST | Locked | LRH-015, LRH-017 |
| LRH-021 | Move notification child process I/O and teardown off the message thread | SHOULD | Locked | LRH-014–015 |
| LRH-022 | Bound the low-level hook diagnostic runtime | COULD | Locked | LRH-019 |
| LRH-023 | Remove unnecessary ready-check JSON copies | COULD | Locked | LRH-016–017 |
| LRH-024 | Audit dependency and release footprint using measurements | COULD | Planned | LRH-014 |

Locked tasks require a concrete dependency completion and recorded validation evidence. Handoffs include changed files, tests run, failures, assumptions, and plan status updates.

## Per-task validation gate

Every task must provide all five items before it may be marked `Complete`:

1. Implementation code for the approved scope.
2. Executable validation code: unit tests, integration tests, a diagnostic command, or a runtime harness appropriate to the task.
3. Automated evidence: formatting, tests, Clippy, build, security, or performance checks applicable to the task.
4. Exact user-run instructions for the real behavior.
5. Recorded evidence in this plan and `TRACEABILITY.md`, including failures and remaining limitations.

Compilation alone never satisfies the gate. A task remains `Validation-required` until the user-run scenario has been completed or the task has an explicitly documented, reproducible substitute.

Phase 5 selection: LRH-009 first, then LRH-010. LRH-011 and LRH-012 remain locked until the notification window is integrated and tested.

Phase 7 completion: LRH-014 passed automated diagnostics and release checks, and the user explicitly confirmed the remaining real Windows validation gate on 2026-08-23.

Phase 8 selection: LRH-019 is implemented and awaits its real Windows input/ready-check validation. LRH-015 is the highest-RICE unblocked implementation task; LRH-024 is also unblocked. Correctness, explicit-action safety, cleanup, credential secrecy, and validation dependencies override score. JSON and dependency work remains measurement-gated and is skipped when benefit is immaterial.
