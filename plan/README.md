# Delivery plan

Active phase: Phase 1 — Foundation and authenticated LCU connection.

Selection rule: choose the highest-RICE unblocked task in the active phase; dependencies, security gates, and human decisions always take precedence.

| ID | Task | Priority | Status | Dependencies |
|---|---|---|---|---|
| LRH-001 | Bootstrap repository and Rust application | MUST | Complete | — |
| LRH-002 | Discover and authenticate to LCU securely | MUST | Validation-required | LRH-001 |
| LRH-003 | Consume and model ready-check state | MUST | Locked | LRH-002 |
| LRH-004 | Implement conditional Windows hotkeys | MUST | Locked | LRH-001, LRH-003 |
| LRH-005 | Implement accept/decline commands | MUST | Locked | LRH-002–004 |
| LRH-006 | Add tray lifecycle and startup toggle | SHOULD | Locked | LRH-005 |
| LRH-007 | Harden reconnect, shutdown, and secrets | MUST | Locked | LRH-006 |
| LRH-008 | Validate and package portable release | MUST | Locked | LRH-007 |

Locked tasks require a concrete dependency completion and recorded validation evidence. Handoffs include changed files, tests run, failures, assumptions, and plan status updates.
