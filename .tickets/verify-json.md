---
status: closed
priority: 3
deps: [cmd-verify]
attachments: []
created: 2026-11-10T06:39:07Z
updated: 2026-11-10T07:53:32Z
closed-at: 2026-11-10T07:53:32Z
---
# verify --json output

Machine-readable matches for CI, per spec §5.4.

Scope:
- `--json` on `verify` writes the match list as JSON to stdout instead of the human block.
- Each match carries `id`, `description`, `target`, the matched value, and a line number where one applies. Identity matches carry no line number.
- Exit codes are identical to the human path for the same message. The flag changes the format only, which is what spec §5.4 states.
- Errors still go to stderr with exit 2, so a CI job never parses an error as an empty match list.

Done when:
- A test parses the output and asserts one object per match.
- Exit code equals the human path for the same message, on both the clean and the match path.
- An empty match list is valid JSON and exits 0.
- An error path writes nothing to stdout, so JSON consumers never see a partial document.
