# AGENTS.md

## Project
`commit-msg-attribution-checker` rejects a git commit message that records
LLM-agent attribution. Installed as a `commit-msg` hook, its exit status
aborts the commit; the same binary run over a pushed range in CI is the
authoritative check. The tool answers one question only: does this message
contain a marker that the configured pattern list rejects? It does not infer
AI use from code, from wording without a marker, or from branch names.

Stack: Rust (edition 2024), plain `cargo`, no workspace and no build wrapper.
No database, no server, no network calls.

## Starting a session
Work lives in `bl` tickets: one committed `backlog.md` derived from `.tickets/`,
so the tracker stays in the repo and never drifts from the code.
- `bl available` — claimable tickets (open, deps satisfied)
- `bl started` — tickets in progress
- `bl show <id> --json` — one ticket in full
- `bl help` — the full command surface

## Commands
- Setup: `cargo build` (rustc/cargo 1.99.1; versions are pinned in `Cargo.lock`)
- Tests: `cargo test` (unit tests in `src/`, integration tests in `tests/`)
- Format: `cargo fmt --check`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Build: `cargo build`
- Install: `make install` — release build copied to `~/.local/bin`. Manual and
  outside `check`, so verification never writes outside `target/`.
- **Full verification: `make check`** — runs fmt, clippy and tests, then prints
  one verdict line.

## Definition of Done
A ticket is done only when:
- `make check` is green, including the tests that existed before this session.
- The ticket's own "Done when" checks pass.
- `doc/spec.md` still describes the behaviour the code has.
- The ticket is closed with `bl done <id>`.

NOT done: "code is written", "looks right", "the unit tests pass".

## Hard constraints (MUST / MUST NOT)
1. MUST: `doc/spec.md` is the source of truth. A behaviour change edits the
   spec first, then the code. Reason: the spec is what the hook contract and
   the acceptance criteria are reviewed against, so code that drifts from it
   changes the contract without anyone agreeing.
2. MUST NOT: `config create` overwrites an existing file. It fails with exit 2,
   prints the path, and leaves the file byte-identical (spec §6). Reason: the
   pattern list is hand-edited, so overwriting it destroys a user's work.
3. MUST: every pattern carries a `description` that states what it rejects.
   Reason: the description is what the rejection message prints; a pattern
   without one does not tell the user why the commit was blocked.
4. MUST: exit codes are only `0` (clean), `1` (attribution found), and `2`
   (usage, I/O or config error), per spec §6. Reason: the hook and CI branch on
   the status, and any other code is read as one of those three.
5. MUST NOT: add a crate. The crate set is exactly the five in spec §10:
   `clap`, `regex`, `serde`, `toml`, `dirs`. Adding one is a spec change: file
   a ticket, do not edit `Cargo.toml` first.

## Work rules
- **WIP=1.** Claim one ticket at a time with `bl start <id>`, and close it with
  `bl done <id>` before claiming another.
- Stay inside the claimed ticket. Record any other problem with
  `bl add "<title>"` and continue; do not fix it in passing.
- Do not edit `.tickets/` or `backlog.md` by hand. `bl` owns them, and
  `backlog.md` is regenerated from the detail files.
- Priority order: correctness → clarity → performance.
- Commit messages: one-line topic; a blank line and a short paragraph only
  when necessary; no attribution trailers (the tool in this repo rejects them).
- Comments state the reason for the code, not a restatement of it.

## Escalate to human (stop and ask)
- Changing behaviour that `doc/spec.md` defines. Write the spec edit first and
  get agreement, because tickets and code are written against it.
- Adding a crate, or any external dependency or service.
- Adding, weakening or removing a default pattern: users rely on the set.
- Changing an exit code, the report format, or the config schema.
- Deleting anything that cannot be recovered (tickets, history, user files).

## Details (read when needed)
- `doc/spec.md`: the contract for every ticket — matching (§5), commands and
  exit codes (§6), default patterns (§4), crate set (§10).
- `doc/llm_commit_attribution_filtering.md`: the research the default patterns
  come from. Read before changing or adding one.
- `README.md`: one line of summary, plus spec §7's hook snippet, which
  `.git/hooks/commit-msg` must carry verbatim.
- This checkout installs the hook in `.git/hooks/commit-msg`, running this
  build's binary, so here a commit that records attribution is blocked, and a
  deleted `target/` blocks every commit until you rebuild. `.git/hooks/` is
  untracked, so a clone has none. Bypass one commit with
  `git commit --no-verify` (spec §2).
- `docs/`: does not exist yet. Create it only when a section here outgrows the
  150-line budget, then leave a pointer to it.
