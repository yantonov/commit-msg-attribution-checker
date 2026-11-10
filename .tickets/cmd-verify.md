---
status: open
priority: 2
deps: [config-resolution, match-engine]
attachments: []
created: 2026-11-10T06:39:06Z
updated: 2026-11-10T06:40:40Z
---
# verify command: human output and exit codes

Wire the engine into `verify <msg-file>` with the output block from spec §5.4.

Scope:
- Clean message: no output on stdout, exit 0.
- Match: the block on stdout, exit 1, with one stanza per match (id, line and target where they apply, description, value) and a closing line with the count and `Commit rejected.`
- Config, usage, or I/O error: message on stderr, exit 2, nothing on stdout.
- Exit codes are exactly 0, 1, 2 as spec §6 defines them. No other value leaves the process.
- `--config`, `--author`, `--committer`, `-v` honored. `-v` adds the identity-skip note on stderr; it never changes stdout.
- Read the message file once. A missing or unreadable file is exit 2 naming the path.

Acceptance criteria owned here, end to end through the binary: 1, 2, 3, 4, 5, 7, 8.

Done when:
- Criteria 1, 2, 3, 4, 5, 7, 8 pass through the binary.
- Criterion 6 is asserted here too, as a regression on the engine output: `Assisted-by: Amp` reports one match.
- The output block matches spec §5.4 in field order and wording.
- Exit codes are asserted for the clean path, the match path, the missing-file path, and the unsupported-flag path.
