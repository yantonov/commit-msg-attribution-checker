---
status: closed
priority: 2
deps: []
attachments: []
created: 2026-11-10T08:29:37Z
updated: 2026-11-10T08:29:52Z
closed-at: 2026-11-10T08:29:52Z
---
# README: move Motivation below the table of contents
The table of contents sits after Motivation, so its Motivation entry points back up to
a section the reader has already passed. A TOC belongs directly under the summary it
indexes.

Scope:
- Move the Motivation block from above the TOC to directly below it, so the document
  reads: H1, summary, TOC, Motivation, Install as a hook, Or register it with krok.
- The TOC keeps its three entries in the order they appear; only the Motivation block
  moves, so no entry sits above its section.
- The Motivation wording is unchanged. Position only.

Done when:
- TOC is directly under the summary, and every TOC entry points to a section below it.
- The `sh` block keeps spec §7's snippet verbatim.
- `make check` is green.
