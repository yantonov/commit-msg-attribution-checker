---
title: "App placeholder: cargo scaffold"
status: closed
priority: 0
deps: []
attachments: []
created: 2026-11-10T06:38:46Z
updated: 2026-11-10T06:47:03Z
closed-at: 2026-11-10T06:47:03Z
---
# App placeholder: cargo scaffold

Create the Rust crate skeleton so every later ticket builds on a working binary.

Scope:
- `cargo init` layout. `Cargo.toml` pins edition 2024 and exactly the five crates spec §10 names: `clap` (derive feature), `regex`, `serde` (derive feature), `toml`, `dirs`. Adding a crate is a spec change, so file a ticket instead of editing the list.
- `src/main.rs` parses the command shape from spec §6: `verify <msg-file>` and `config <defaults|create|show>`.
- Each subcommand body returns a "not implemented" error on stderr with exit 2, so the exit-code contract from spec §6 already holds.
- `--config <path>` is declared once at the top level as a global flag, accepted by every subcommand, and unused in this ticket.
- Tests live in the crate from this ticket on. Add `tests/` with one smoke test that runs the binary and asserts the `config` subcommand exits 2, so `make test` is never green by having nothing to run.
- Version string from `Cargo.toml` via `clap`.
- No module split yet. One file is enough until `config-model` and `msg-parsing` land.

Done when:
- `cargo build` succeeds and `cargo run -- --help` lists `verify` and `config`.
- `cargo run -- config` exits 2 and writes the subcommand list to stderr, matching spec §6.
- `cargo run -- verify` with no argument exits 2 with a usage error, not a panic.
- `cargo test` runs at least one test and passes.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean.
