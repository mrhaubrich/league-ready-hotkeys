# Agent instructions

This is a Windows-only Rust utility. Keep LCU transport, pure state logic, and Win32 UI in separate modules. Never log lockfile credentials or accept automatically. Register F1/F2 only for an active ready check and always unregister them on disconnect and shutdown. Keep unsafe Win32 calls isolated behind small safe wrappers. Every task must include executable validation code and exact user-run validation instructions. Use the active task in `plan/README.md`; do not bypass dependencies or mark work complete without the task's validation evidence.
