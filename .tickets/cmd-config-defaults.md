---
status: open
priority: 2
deps: [default-patterns]
attachments: []
created: 2026-11-10T06:38:55Z
updated: 2026-11-10T06:39:11Z
---
# config defaults

Print the built-in default config as TOML to stdout.

Scope:
- Renders the registry from `default-patterns` through the same serializer `config create` uses, so the two cannot diverge.
- No header comment naming a source path, because the source is the binary itself.
- Exit 0. Ignores the resolved user config entirely.

Done when:
- `config defaults` output parses as TOML and loads through the config model.
- Output is identical to the file `config create` writes.
- A test asserts the command exits 0 with a user config present and still prints the defaults.
