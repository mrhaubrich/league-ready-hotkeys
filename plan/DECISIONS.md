# Decisions

| Decision | Rationale |
|---|---|
| Rust standalone executable | Low footprint and no runtime installation. |
| Nested repository | Parent `D:\git` contains unrelated projects. |
| Event-driven LCU monitoring | Avoid frequent polling while idle. |
| Conditional F1/F2 | Preserve League's normal F1 behavior outside ready checks. |
| HKCU Run tray toggle | No elevation and portable deployment. |
| Balanced native dependencies | Lower complexity than pure Win32 while keeping resource use small. |
| LCU worker with bounded transport waits | Network and discovery stalls must not block Win32 tray, notification, shortcut, or shutdown messages; explicit action attempts are gated before dispatch and never automatically retried. |
| LCU events plus reconciliation polling | Events reduce ready-check latency, while polling preserves recovery from initial state, missed events, disconnects, and contract drift. |
| Measurement-gated micro-optimization | JSON, dependency, TLS, and binary-footprint changes proceed only when Windows measurements show material benefit. |
