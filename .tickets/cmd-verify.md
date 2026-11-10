---
status: open
priority: 2
deps: [config-resolution, match-engine]
attachments: []
created: 2026-11-10T06:39:06Z
updated: 2026-11-10T06:39:11Z
---
# verify command: human output and exit codes

Wire the engine into `verify <msg-file>` with the output block from spec §5.4.

Scope:
- Clean message: no output, exit 0.
- Match: the block on stdout, exit 1, ending with the match count and `Commit rejected.`
- Config or usage error: message on stderr, exit 2.
- Exit codes are exactly 0, 1, 2 as spec §6 defines them.
- `--config`, `--author`, `--committer`, `-v` honored.

Done when:
- Acceptance criteria 1, 2, 3, 5, 6, 7 pass end to end through the binary.
- Output format matches the spec block: id, line and target, description, value, count.
- Exit codes are asserted for the clean, match, and error paths.
