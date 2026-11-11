---
title: "Config resolution order"
status: closed
priority: 1
deps: [app-placeholder]
attachments: []
created: 2026-11-10T06:38:53Z
updated: 2026-11-10T06:55:10Z
closed-at: 2026-11-10T06:55:10Z
---
# Config resolution order

Implement spec §3.2: the four-step lookup that decides which pattern list runs.

Scope:
- Resolution order, exactly as spec §3.2 states it: `--config <path>`, then `COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG`, then `$XDG_CONFIG_HOME/commit-msg-attribution-checker/config.toml`, then `~/.config/commit-msg-attribution-checker/config.toml`, then the built-in defaults.
- Read `XDG_CONFIG_HOME` from the environment directly. When it is unset, fall back to `<home>/.config`. Do not call `dirs::config_dir()`: on Windows that returns `%APPDATA%`, which contradicts the literal path in spec §3.1. Use `dirs` only to find the home directory.
- An explicit path (flag or env var) that is missing or malformed is exit 2. It never falls back to defaults.
- Return which source won, so `config show` can name it in the header comment.

Done when:
- Tests cover: flag wins over env var, env var wins over file, file wins over defaults, defaults when nothing exists.
- A missing file named by `--config` exits 2 rather than using defaults.
- A missing file named by the env var exits 2.
- A test sets `XDG_CONFIG_HOME` and asserts the file is read from there, and unsets it and asserts the `~/.config` path.
- A test asserts `dirs::config_dir()` is not used for the config path.
