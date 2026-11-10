---
status: closed
priority: 2
deps: []
attachments: []
created: 2026-11-10T08:16:13Z
updated: 2026-11-10T08:19:53Z
closed-at: 2026-11-10T08:19:53Z
---
# Make install target: release artifact then install to ~/.local/bin

Add a public `make install` that depends on a new internal `build-release` target and copies the release binary to $HOME/.local/bin, modelled on ../prosecraft.

Scope:
- `build-release`: `cargo build --release`, producing target/release/commit-msg-attribution-checker (plus .exe on Windows).
- `install: build-release`: mkdir -p $HOME/.local/bin, copy the artifact there, print the installed path.
- The copy must name the platform's real artifact. prosecraft relies on MSYS `cp` auto-appending .exe; this target resolves the artifact explicitly instead.
- install is a prerequisite of nothing and never part of `check`, so verification stays offline and side-effect free.
- `help` and `help-all` list install (public) and build-release (internal); AGENTS.md Commands gains the install line.

Done when:
- `make build-release` produces the release artifact.
- `make install` installs a runnable binary on PATH and prints where it went.
- The installed binary answers `--version` and rejects a Claude trailer with exit 1.
- `make check` stays green and does not depend on install.
- No target leaves a partially copied binary behind when the release build fails.
