---
title: "Match engine with trailer gate"
status: closed
priority: 1
deps: [config-model, msg-parsing]
attachments: []
created: 2026-11-10T06:39:03Z
updated: 2026-11-10T07:05:51Z
closed-at: 2026-11-10T07:05:51Z
---
# Match engine with trailer gate

Run the loaded patterns against the parsed message, per spec §5.2 and §5.3.

Inputs: a loaded `Config` and the parsed message from `msg-parsing`. Output: an ordered list of matches.

Scope:
- Trailer patterns run first on every line.
- `line` and `subject` patterns run on a line unless a trailer pattern already matched that line. The gate is a trailer match, not trailer shape, so `aider: fix validation` still reaches `aider.subject`. Spec §5.2 states this rule and why it exists.
- The `subject` skip is independent of the `line` skip, so a subject line a trailer pattern claimed is skipped for both.
- Record the first match per pattern, with `id`, `description`, `target`, the matched value, and the line number. Report every match, in file order, not just the first.
- Regexes are compiled once at config load. No `git interpret-trailers` call, per spec §5.2.
- Ordering of the returned matches is stable, so the output block and the JSON are deterministic.

Done when:
- Acceptance criteria 1, 3, 5, 6, 7 pass as unit tests.
- A message with three attribution lines yields three matches.
- `Assisted-by: Amp <amp@ampcode.com>` yields one match, not two.
- A subject `aider: fix validation` yields `aider.subject`.
- A test asserts two runs over the same input produce identical order.
