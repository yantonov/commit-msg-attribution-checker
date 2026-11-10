---
status: open
priority: 1
deps: [app-placeholder]
attachments: []
created: 2026-11-10T06:38:53Z
updated: 2026-11-10T06:39:11Z
---
# Config resolution order

Implement spec §3.2: the four-step lookup that decides which pattern list runs.

Scope:
- Resolution order: `--config <path>`, then `COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG`, then `$XDG_CONFIG_HOME/commit-msg-attribution-checker/config.toml`, then `~/.config/commit-msg-attribution-checker/config.toml`, then built-in defaults.
- `dirs` resolves `XDG_CONFIG_HOME` and the home directory, so Windows works without reading `HOME`.
- An explicit path (flag or env var) that is missing or malformed is exit 2. It never falls back to defaults.
- Return which source won, so `config show` can name it in the header comment.

Done when:
- Tests cover: flag wins over env var, env var wins over file, file wins over defaults, defaults when nothing exists.
- A missing file named by `--config` exits 2 rather than using defaults.
- A missing file named by the env var exits 2.
- Resolution works when `XDG_CONFIG_HOME` is set and when it is unset.
