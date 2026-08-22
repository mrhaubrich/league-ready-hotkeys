# Traceability

| Goal | Phase | Tasks | Code/tests/evidence |
|---|---|---|---|
| F1 accepts and F2 declines | 2 | LRH-003–005 | `src/app.rs`, `src/lcu`, `src/windows`, ready-check integration tests |
| Secure LCU session boundary | 1 | LRH-002 | `src/lcu/discovery.rs`, `src/lcu/transport.rs`, parser tests |
| No accidental in-game capture | 2/3 | LRH-004, LRH-007 | hotkey lifecycle tests and manual Windows scenario |
| Starts with Windows | 3 | LRH-006 | `src/windows/startup.rs`, registry manual check |
| Low idle usage | 4 | LRH-007/008 | reconnect tests and measured release footprint |
