# Delivery plan

Active phase: Phase 2 — Ready-check vertical slice.

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
| LRH-008 | Validate and package portable release | MUST | Validation-required | LRH-007 |

Locked tasks require a concrete dependency completion and recorded validation evidence. Handoffs include changed files, tests run, failures, assumptions, and plan status updates.

## Per-task validation gate

Every task must provide all five items before it may be marked `Complete`:

1. Implementation code for the approved scope.
2. Executable validation code: unit tests, integration tests, a diagnostic command, or a runtime harness appropriate to the task.
3. Automated evidence: formatting, tests, Clippy, build, security, or performance checks applicable to the task.
4. Exact user-run instructions for the real behavior.
5. Recorded evidence in this plan and `TRACEABILITY.md`, including failures and remaining limitations.

Compilation alone never satisfies the gate. A task remains `Validation-required` until the user-run scenario has been completed or the task has an explicitly documented, reproducible substitute.
