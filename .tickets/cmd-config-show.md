---
status: closed
priority: 2
deps: [config-resolution]
attachments: []
created: 2026-11-10T06:38:58Z
updated: 2026-11-10T07:21:00Z
closed-at: 2026-11-10T07:21:00Z
---
# config show

Print the effective config, with a header comment naming the source that won. Contract: spec §6.

Scope:
- Uses the resolution order from `config-resolution`: the file in use, or the defaults, per spec §3.2.
- Header comment names the source path, or states that the built-in defaults apply.
- Output is TOML that loads back through the config model.
- The body is the effective pattern list, not a copy of the file text: re-serialize the loaded `Config`, so the output is valid TOML even when the user hand-edited the file with unusual formatting.

Done when:
- With a user config present, the emitted pattern list equals the patterns from that file and the header names its path.
- With no user config, the emitted pattern list equals `config defaults` and the header states that the defaults apply.
- Both outputs load through the config model.
- A test uses a user file with odd formatting and asserts the output still parses.
