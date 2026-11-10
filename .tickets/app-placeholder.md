---
status: open
priority: 0
deps: []
attachments: []
created: 2026-11-10T06:38:46Z
updated: 2026-11-10T06:38:46Z
---
# App placeholder: cargo scaffold

Create the Rust crate skeleton so every later ticket builds on a working binary.

Scope:
- `cargo init` layout, `Cargo.toml` with edition 2024 and the crate set from spec §10: `clap` (derive), `regex`, `serde` (derive), `toml`, `dirs`.
- `src/main.rs` parses the command shape from spec §6: `verify <msg-file>` and `config <defaults|create|show>`. Every subcommand runs and prints a placeholder line, or returns a "not implemented" error with exit 2.
- `--config <path>` declared as a global flag and accepted but unused.
- Version string from `Cargo.toml` via `clap`.

Done when:
- `cargo build` succeeds and `cargo run -- --help` lists both commands.
- `cargo run -- config` exits 2 with the subcommand list.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean.
