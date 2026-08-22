# Traceability

| Goal | Phase | Tasks | Code/tests/evidence |
|---|---|---|---|
| F1 accepts and F2 declines | 2 | LRH-003–005 | `src/app.rs`, `src/lcu`, `src/windows`, ready-check integration tests |
| No accidental in-game capture | 2/3 | LRH-004, LRH-007 | hotkey lifecycle tests and manual Windows scenario |
| Starts with Windows | 3 | LRH-006 | `src/windows/startup.rs`, registry manual check |
| Low idle usage | 4 | LRH-007/008 | reconnect tests and measured release footprint |
