---
status: closed
priority: 2
deps: [cmd-config-create, cmd-verify]
attachments: []
created: 2026-11-10T06:40:47Z
updated: 2026-11-10T07:51:01Z
closed-at: 2026-11-10T07:51:01Z
---
# End-to-end acceptance suite

Prove the round-trip claims that no single unit ticket owns: creating a config changes nothing about behaviour, and replacing the defaults changes everything. Contract: spec §3.3 and acceptance criteria 9 and 11.

This ticket exists because the two remaining spec acceptance criteria need both `config create` and `verify` in the same test, and neither of those tickets should depend on the other.

Scope:
- A `tests/` integration test that runs the built binary twice against a temporary config path.
- Case A, criterion 9: `config create`, then `verify` a message with `Co-authored-by: Claude <noreply@anthropic.com>`. Expect exit 1, because the created file is the full default set and behaves identically to the built-ins.
- Case B, criterion 11: write a hand-made user config containing one unrelated pattern, then `verify` the same message. Expect exit 0, because replacement semantics disable every built-in the file omits.
- Both cases assert against the same message fixture, so the only variable is the config file.
- No production code changes. If a case fails, file a ticket against the owning component rather than patching here.

Done when:
- Criteria 9 and 11 both pass.
- The test fails if replacement semantics ever become merge semantics, which is the behaviour spec §3.3 makes deliberate.
- The test touches no real config path and no real home directory.
