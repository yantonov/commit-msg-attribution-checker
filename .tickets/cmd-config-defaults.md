---
status: open
priority: 2
deps: [default-patterns]
attachments: []
created: 2026-11-10T06:38:55Z
updated: 2026-11-10T06:40:32Z
---
# config defaults

Print the built-in default config as TOML to stdout.

Scope:
- Renders the registry from `default-patterns` through the same serializer `config create` uses, so the two outputs cannot diverge.
- No header comment naming a source path: the source is the binary, and spec §6 gives the header to `config show` only.
- Exit 0. Ignores the resolved user config entirely, per spec §6.
- An I/O failure writing to stdout is exit 2.

Done when:
- The output parses as TOML and loads through the config model.
- The output is byte-identical to the file `config create` writes, so the two tickets share one serializer test.
- A test asserts the command exits 0 and prints the defaults even when a user config exists.
- A test asserts no header comment is present, because that distinguishes this command from `config show`.
