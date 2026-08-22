# Decisions

| Decision | Rationale |
|---|---|
| Rust standalone executable | Low footprint and no runtime installation. |
| Nested repository | Parent `D:\git` contains unrelated projects. |
| Event-driven LCU monitoring | Avoid frequent polling while idle. |
| Conditional F1/F2 | Preserve League's normal F1 behavior outside ready checks. |
| HKCU Run tray toggle | No elevation and portable deployment. |
| Balanced native dependencies | Lower complexity than pure Win32 while keeping resource use small. |
