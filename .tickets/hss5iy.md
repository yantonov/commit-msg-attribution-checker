---
status: closed
priority: 2
deps: []
attachments: []
created: 2026-11-10T08:24:17Z
updated: 2026-11-10T08:25:06Z
closed-at: 2026-11-10T08:25:06Z
---
# Add a Motivation section to the README

README gains a short Motivation section stating why the tool exists, before the install sections.

The argument to make, accurately:
- Attribution trailers and footers name the tool that produced a change. They do not describe the change. A reader of history has to filter them out to find the subject, the reasoning and the review context that a commit is for.
- Attribution is per-tool and per-session. It accumulates with every agent, editor and harness in use, so the noise grows while the information does not: two commits with identical content read differently only because of which tool touched them.
- The same class of trailer is already used for things that have nothing to do with the change: co-authoring conventions credit an operating system, a hardware vendor or another tool. That is the point: a co-authored-by or made-with line says who or what was involved in producing a commit, which is metadata about the making, not about the merit or content of the change.
- So the tool treats the class as noise on the local commit message, and rejects a message that carries it. It makes no claim about how the code was written.

Constraints:
- Do not overclaim: the tool does not detect AI authorship, does not judge code, and spec §2 already says a match only means a configured pattern matched. The section must not imply it identifies AI-written code.
- Keep it short: a paragraph or two, plus at most one short list. It is a motive, not a manifesto.
- Do not restate spec §2's non-goals or the hook/soft-gate paragraph; those stay where they are.
- Place it after the one-line summary and before the table of contents, or as the first TOC entry, so a reader meets it before installing.

Done when:
- The README opens with the why, and the TOC names the new section.
- The section states the noise argument, that attribution describes the making rather than the change, and the co-authored-by-for-OS-or-hardware parallel.
- It makes no claim about detecting AI authorship, and the existing hook and krok sections are unchanged.
- make check stays green (the README snippet test must still pass).
