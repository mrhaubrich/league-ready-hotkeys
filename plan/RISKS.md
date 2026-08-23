# Risks

| Risk | Likelihood | Impact | Mitigation | Owner | Status |
|---|---:|---:|---|---|---|
| LCU event/API changes | Medium | High | Runtime fallback GET and contract fixtures | LCU integration | Open |
| Credential leakage | Low | High | Redaction, no persistence, review errors/logging | Reviewer | Open |
| Stale global hotkeys | Medium | High | State-gated registration and unconditional cleanup | Windows integration | Open |
| Moved portable executable | Medium | Medium | Tray status and startup repair action | Windows integration | Open |
| Toolchain unavailable | High | Medium | Install stable MSVC Rust before build validation | Validator | Open |
| Stalled LCU work starves Windows messages | Medium | High | Dedicated worker, bounded timeouts, stalled-loopback diagnostics, and UI latency traces | Runtime integration | Planned LRH-015 |
| Asynchronous or retried action sends duplicate Accept/Decline | Low | High | Claim the action gate before dispatch, attach ready-check generations, and never automatically retry POST actions | Runtime integration | Planned LRH-015/LRH-021 |
| Event loss or reconnect leaves stale ready-check state | Medium | High | Complement events with reconciliation polling, idempotent transitions, and disconnect cleanup | LCU integration | Planned LRH-017 |
| Hook optimization changes matching or input pass-through | Medium | High | Binding-equivalence tests, callback timing, high-rate input tests, and real ready-check cleanup evidence | Windows integration | Automated mitigation passed; LRH-019 user validation required |
| Timeout is shorter than a healthy local LCU response | Medium | High | Derive thresholds from Windows loopback traces and validate during League restart/load | Validator | Planned LRH-015 |
| Delayed worker or child message revives stale UI/actions | Low | High | Generation-check every completion/IPC response against authoritative active state | Runtime integration | Planned LRH-015/LRH-021 |
