---
status: open
priority: 1
deps: [config-model]
attachments: []
created: 2026-11-10T06:38:54Z
updated: 2026-11-10T06:39:11Z
---
# Default pattern registry

The 22 built-in patterns from spec §4, as one constant list that both `config defaults` and `config create` render.

Scope:
- One Rust source file holding the pattern records for §4.1 to §4.6, each with its `id`, `description`, `target`, `key`, and `regex`, exactly as the spec tables list them.
- Serialization to TOML, so the emitted file round-trips through the config model from `config-model`.
- Ordering stable and grouped as the spec groups them, so a reader can compare the emitted file against the spec.
- Identities as the spec states them: bare-word matches for `aider`, `amp`, `gemini`, domain-anchored aliases elsewhere.

Done when:
- The emitted TOML loads back through the config model with no error.
- A test asserts all 22 ids are present.
- A test asserts every pattern has a non-empty `description`, because the rejection message prints it.
- The free-form footer appears as three patterns (`emoji`, `tool`, `url`), not one.
