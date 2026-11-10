---
status: open
priority: 1
deps: [app-placeholder]
attachments: []
created: 2026-11-10T06:38:51Z
updated: 2026-11-10T06:39:11Z
---
# Config model and schema validation

Typed representation of spec §3.4, so every other ticket loads config through one path.

Scope:
- `Pattern` struct: `id`, `description`, `target`, `key` (optional), `regex`. `Target` enum: `trailer`, `line`, `subject`, `author`, `committer`.
- `Config` struct with `version` (informational) and `patterns`.
- Deserialize from TOML with `serde`. `deny_unknown_fields` everywhere, because spec §3.4 makes an unknown field an exit-2 config error.
- Validate after load: `id` unique within the file, `key` present exactly when `target` is `trailer`, every `regex` compiles. Report the first violation with the offending `id` and exit 2.
- Compile regexes once at load, not per match.

Done when:
- Unit tests cover: valid config loads; unknown field rejected; duplicate `id` rejected; missing `key` on a `trailer` pattern rejected; `key` on a `line` pattern rejected; invalid regex rejected; unknown `version` accepted.
- Every rejection produces exit 2 and a message naming the pattern.
