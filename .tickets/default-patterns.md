---
title: "Default pattern registry"
status: closed
priority: 1
deps: [config-model]
attachments: []
created: 2026-11-10T06:38:54Z
updated: 2026-11-10T06:59:17Z
closed-at: 2026-11-10T06:59:17Z
---
# Default pattern registry

The 22 built-in patterns from spec §4, as one constant list that both `config defaults` and `config create` render.

Scope:
- One Rust source file holding the pattern records for spec §4.1 to §4.6, each with its `id`, `description`, `target`, `key`, and `regex`, exactly as the spec tables list them. The spec tables are the source of truth; a disagreement is a spec bug, so fix the spec first.
- 22 records in total, in this exact set. §4.1: `co-authored-by.claude`, `.codex`, `.copilot`, `.cursor`, `.amp`, `.aider`, `.opencode`, `.gemini`. §4.2: `assisted-by`, `generated-by`, `ai-assisted-by`, `made-with`. §4.3: `claude-session`, `amp-thread-id`. §4.4: `free-form-footer.emoji`, `.tool`, `.url`. §4.5: `aider.author-name`, `aider.committer-name`, `aider.subject`. §4.6: `author.agent-identity`, `committer.agent-identity`.
- Serialization to TOML, so the emitted file round-trips through the config model from `config-model`.
- Ordering stable and grouped as the spec groups them, so a reader can compare the emitted file against the spec table by table.
- Identities as the spec states them: bare-word matches for `aider`, `amp`, `gemini`, domain-anchored aliases elsewhere. Do not tighten a pattern while implementing it; that is a spec change.

Done when:
- The emitted TOML loads back through the config model with no error.
- A test asserts the id set above exactly, so adding or dropping a pattern fails rather than passing quietly.
- A test asserts every pattern has a non-empty `description`, because the rejection message prints it.
- The free-form footer appears as three records, not one.
- A test asserts `aider`, `amp`, and `gemini` still match on the bare word, so an accidental tightening fails.
