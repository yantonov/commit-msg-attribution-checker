---
status: open
priority: 2
deps: [config-resolution]
attachments: []
created: 2026-11-10T06:38:58Z
updated: 2026-11-10T06:39:11Z
---
# config show

Print the effective config, with a header comment naming the source that won.

Scope:
- Uses the resolution order from `config-resolution`: file in use, or the defaults.
- Header comment names the source path, or states that the built-in defaults apply.
- Output is TOML that loads back through the config model.

Done when:
- With a user config present, the output matches that file and the header names its path.
- With no user config, the output matches `config defaults` plus the defaults header.
- Both outputs load through the config model.
