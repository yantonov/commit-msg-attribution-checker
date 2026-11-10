---
status: open
priority: 1
deps: [config-model, msg-parsing]
attachments: []
created: 2026-11-10T06:39:03Z
updated: 2026-11-10T06:39:11Z
---
# Match engine with trailer gate

Run the loaded patterns against the parsed message, per spec §5.2 and §5.3.

Scope:
- Trailer patterns run first on every line.
- `line` and `subject` patterns run on a line unless a trailer pattern already matched that line. The gate is a trailer match, not trailer shape, so `aider: fix validation` still reaches `aider.subject`.
- The `subject` skip is independent of the `line` skip.
- Record the first match per pattern, with `id`, `description`, `target`, matched value, and line number. Report all matches, not just the first.
- Compile regexes once. No `git interpret-trailers` call.

Done when:
- Acceptance criteria 1, 3, 5, 6, 7 pass as unit tests.
- A message with three attribution lines yields three matches.
- `Assisted-by: Amp <amp@ampcode.com>` yields one match, not two.
- A subject `aider: fix validation` yields `aider.subject`.
