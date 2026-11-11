---
title: "verify command: human output and exit codes"
status: closed
priority: 2
deps: [config-resolution, match-engine]
attachments: []
created: 2026-11-10T06:39:06Z
updated: 2026-11-10T07:47:27Z
closed-at: 2026-11-10T07:47:27Z
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

Blocked on a spec contradiction (needs a human decision before this ticket can close):
- Criterion 4 second half FAILS: "The same text with two leading spaces exits 1". Observed exit 0. The code matches §5.1 and §5.2; the criterion contradicts them. §5.1 keeps the line (only a first-column '#' is a comment), so it reaches matching; but §5.2's trailer target tests lines matching ^([A-Za-z0-9][A-Za-z0-9-]*):[ \t]*(.+)$, which cannot match a line with two leading spaces. So the line has no trailer shape, and no shipped line/subject pattern claims it.
- Pinned by tests/verify.rs::pin_an_indented_co_author_trailer_does_not_report_today and ::an_indented_co_author_trailer_carries_no_trailer_shape.
- Everything else in scope passes: criteria 1, 2, 3, 5, 6, 7, 8 through the binary, criterion 12 through the binary, and the §5.4 block byte-identical to the spec example. --json is rejected with exit 2, as verify-json owns it.
- Resolution options: (a) amend criterion 4 to say the indented text exits 0, matching §5.1 and §5.2 as written; (b) amend §5.2 to allow leading whitespace in the trailer expression, which changes matching for every trailer pattern; (c) add a line pattern that catches indented attribution. (a) is the smallest edit and changes no behaviour.

Resolved: a human chose option (a), so this ticket closes.
- doc/spec.md §5.1 gained a paragraph stating that a line which is not a comment is not thereby claimed, and §8 criterion 4 now says the indented text exits 0.
- The pinning test is renamed an_indented_co_author_trailer_is_clean; it now asserts the settled contract rather than documenting a conflict.
- No behaviour changed: the code already matched §5.1 and §5.2.
