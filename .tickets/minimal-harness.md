---
status: open
priority: 1
deps: [app-placeholder]
attachments: []
created: 2026-11-10T06:38:48Z
updated: 2026-11-10T06:39:11Z
---
# Minimal harness: AGENTS.md and Makefile

Bootstrap the agent harness on top of the working placeholder binary, so every later ticket can be verified with one command.

Scope:
- `AGENTS.md` in the repo root, under 150 lines, following the minimal-harness template. Points at `bl` for work, at `doc/spec.md` for the contract, and at `doc/llm_commit_attribution_filtering.md` for pattern provenance. Records the hard constraints: spec is the source of truth, `config create` never overwrites, patterns carry descriptions, exit codes 0/1/2 are fixed.
- `Makefile` with public `check` and internal `fmt`, `lint`, `test`, `build`. `check` runs fmt + clippy + test and prints one verdict. `help` shows public targets only, `help-all` shows both.
- `docs/` created only if a section outgrows the 150-line AGENTS.md budget.

Done when:
- `make check` runs green on the placeholder binary.
- The cold start test passes: the five questions in the minimal-harness skill are all answerable from `AGENTS.md` alone.
- No `TODO` placeholder remains in either file.
