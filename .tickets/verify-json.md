---
status: open
priority: 3
deps: [cmd-verify]
attachments: []
created: 2026-11-10T06:39:07Z
updated: 2026-11-10T06:39:11Z
---
# verify --json output

Machine-readable matches for CI, per spec §5.4.

Scope:
- `--json` on `verify` emits the match list as JSON instead of the human block.
- Each match carries `id`, `description`, `target`, matched value, and line number where one applies.
- Exit codes unchanged by the flag.

Done when:
- A test parses the output and asserts one object per match.
- Exit code is the same as the human path for the same message.
- An empty match list is valid JSON and exits 0.
