---
status: open
priority: 3
deps: [hook-integration]
attachments: []
created: 2026-11-10T06:39:10Z
updated: 2026-11-10T06:39:11Z
---
# Dogfood the checker on this repo

Install the finished binary as this repository's own `commit-msg` hook and confirm it accepts the project's commit style.

Scope:
- Install the hook locally (not committed) and run `make check` green.
- Confirm the spec and backlog commit messages pass, since they name tools such as Claude, Codex, and Cursor in prose. If a legitimate message is rejected, file a ticket rather than weakening a pattern silently.
- Record in `AGENTS.md` that the hook is installed and how to bypass it for an intentional test commit.

Done when:
- A normal commit on this repo succeeds with the hook installed.
- Any false positive found is filed as its own ticket with the offending message text.
- `make check` stays green with the hook in place.
