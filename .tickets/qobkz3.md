---
status: in_progress
priority: 2
deps: []
attachments: []
created: 2026-11-10T08:28:23Z
updated: 2026-11-10T08:28:28Z
---
# README: title the tool, fix the summary line, shorten Motivation
The README opens with two lines that are both wrong in the same way: neither says
what the tool is. `# commit-msg-attribution-checker` is absent, and the summary
reads "CLI tool to validate commits", which describes any linter and says
nothing about attribution. The Motivation section that follows then runs to
five paragraphs, roughly 250 words, restating one point: attribution records
how a commit was made, not what it changes.

Scope:
- The H1 is the tool name, `# commit-msg-attribution-checker`. This is also the
  name in spec §3.1: a reader arriving from a hook script or a CI log greps for
  it.
- The summary line under it names the subject, not the shape of the binary: it
  rejects a commit message that records LLM-agent attribution. Keep it one line.
- Motivation shrinks to the no-added-value argument: the trailer names the tool
  and the session, not the change, so the noise grows while the information
  does not. One short paragraph, or two sentences plus the co-authored-by-for-
  hardware-or-OS parallel, which is the part that shows the class is not about
  AI. Drop the paragraph that walks through the pattern list except for the one
  clause stating that a match means a configured pattern matched and a
  co-authored-by naming a person passes; that clause is the non-overclaim
  guard and must survive.
- The TOC, the hook section and the krok section are unchanged.

Done when:
- The first line is `# commit-msg-attribution-checker` and the second paragraph
  says what the tool rejects.
- Motivation fits in one paragraph under about 80 words and still carries: the
  trailer describes the making, not the change; the noise grows and the
  information does not; the same class already credits an OS or a hardware
  vendor; a match means a pattern matched, not that an agent wrote the code.
- The `sh` block keeps spec §7's snippet verbatim, so the README snippet test
  in `tests/hook.rs` still passes.
- `make check` is green.

Out of scope: `doc/spec.md` needs no edit, because no behaviour changes.
