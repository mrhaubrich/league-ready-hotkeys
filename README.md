# League Ready Hotkeys

A small Windows tray utility for explicit League Client ready-check actions:

- F1 accepts the active ready check.
- F2 declines the active ready check.
- Keys are captured only while a ready check is active.

The League Client API is a local, unsupported interface. The application must never log or persist the lockfile password, and it must never auto-accept a match.

## Development

Requires the stable Rust MSVC toolchain. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`.

The current foundation parses an LCU lockfile and provides the Windows-only authenticated transport boundary. No credentials are written to disk or included in diagnostics.
