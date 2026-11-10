---
status: closed
priority: 1
deps: [app-placeholder]
attachments: []
created: 2026-11-10T06:38:51Z
updated: 2026-11-10T06:53:03Z
closed-at: 2026-11-10T06:53:03Z
---
# Config model and schema validation

Typed representation of spec §3.4, so every other ticket loads config through one path.

Scope:
- `Pattern` struct: `id`, `description`, `target`, `key` (optional), `regex`. `Target` enum: `trailer`, `line`, `subject`, `author`, `committer`, deserialized from the lowercase names in spec §3.4.
- `Config` struct with `version` (informational, optional) and `patterns`.
- Deserialize from TOML with `serde`, `deny_unknown_fields` on every struct, because spec §3.4 makes an unknown field an exit-2 config error.
- Validate after deserialization: `id` unique within the file, `key` present exactly when `target` is `trailer`, every `regex` compiles. Report the first violation with the offending `id` and exit 2.
- Compile every regex once here and keep the compiled form, so the match engine never recompiles.
- A file with zero `[[pattern]]` entries is valid and yields no patterns; spec §3.3 makes that a supported state, not an error.

Done when:
- Unit tests cover: valid config loads; unknown field rejected; duplicate `id` rejected; missing `key` on a `trailer` pattern rejected; `key` on a `line` pattern rejected; invalid regex rejected; unknown `version` value accepted; empty pattern list accepted.
- Every rejection produces exit 2 and a message naming the offending pattern or field.
- A test asserts the compiled regex list has one entry per pattern.
