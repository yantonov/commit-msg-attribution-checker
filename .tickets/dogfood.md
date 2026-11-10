---
status: open
priority: 3
deps: [hook-integration]
attachments: []
created: 2026-11-10T06:39:10Z
updated: 2026-11-10T06:41:10Z
---
# Dogfood the checker on this repo

Install the finished binary as this repository own `commit-msg` hook and confirm it accepts the project commit messages. Motivation: spec §2 states the hook is a weak gate, and this repo is the first place its false positives would show.

Scope:
- Install the hook locally only. Do not commit `.git/hooks/`.
- Run `make check` green with the hook installed.
- Attempt a commit whose message names tools the way this repo real messages do, such as the spec commit and backlog commit already in history. Those messages discuss Claude, Codex, Cursor, and Amp in prose, so a footer pattern that fires on them is a false positive, not a violation.
- If a legitimate message is rejected, file a ticket with the offending message text and the pattern id. Do not weaken a pattern in this ticket, and do not edit `doc/spec.md` here.
- Record in `AGENTS.md` that the hook is installed in this checkout and how to bypass it for an intentional test commit, so a later agent is not surprised by a blocked commit.

Done when:
- A normal commit on this repo succeeds with the hook installed.
- The two existing commit messages in history pass `verify` when replayed as fixture files.
- Any false positive found is filed as its own ticket with the message text and the pattern id.
- `make check` is green with the hook in place.
