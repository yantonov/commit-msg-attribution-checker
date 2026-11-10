---
status: open
priority: 1
deps: [app-placeholder]
attachments: []
created: 2026-11-10T06:38:48Z
updated: 2026-11-10T06:40:26Z
---
# Minimal harness: AGENTS.md and Makefile

Bootstrap the agent harness on top of the working placeholder binary, so every later ticket can be verified with one command.

Scope:
- `AGENTS.md` in the repo root, under 150 lines, following the minimal-harness template. Points at `bl` for work, `doc/spec.md` for the contract, and `doc/llm_commit_attribution_filtering.md` for pattern provenance.
- Hard constraints recorded in `AGENTS.md`: `doc/spec.md` is the source of truth, so a behaviour change edits the spec first; `config create` never overwrites; every pattern carries a `description`; exit codes are only 0, 1, and 2; the crate set is the five crates in spec §10 and adding one is a spec change.
- `Makefile` with public `check`, plus internal `fmt`, `lint`, `test`, `build`. `check` runs fmt, clippy with `-D warnings`, and test, then prints one verdict. `help` lists public targets only; `help-all` lists both.
- `README.md` stays as-is; the hook snippet belongs to `hook-integration`.
- `docs/` created only if a section outgrows the 150-line budget.

Done when:
- `make check` is green on the placeholder binary.
- The cold start test passes: the five questions in the minimal-harness skill are answerable from `AGENTS.md` alone.
- No `TODO` placeholder remains in `AGENTS.md` or the `Makefile`.
- A test or script asserts no `TODO` remains, so the harness cannot regress silently.
