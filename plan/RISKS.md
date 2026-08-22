# Risks

| Risk | Likelihood | Impact | Mitigation | Owner | Status |
|---|---:|---:|---|---|---|
| LCU event/API changes | Medium | High | Runtime fallback GET and contract fixtures | LCU integration | Open |
| Credential leakage | Low | High | Redaction, no persistence, review errors/logging | Reviewer | Open |
| Stale global hotkeys | Medium | High | State-gated registration and unconditional cleanup | Windows integration | Open |
| Moved portable executable | Medium | Medium | Tray status and startup repair action | Windows integration | Open |
| Toolchain unavailable | High | Medium | Install stable MSVC Rust before build validation | Validator | Open |
