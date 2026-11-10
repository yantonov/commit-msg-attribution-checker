---
status: closed
priority: 3
deps: [cmd-verify]
attachments: []
created: 2026-11-10T06:39:08Z
updated: 2026-11-10T07:58:28Z
closed-at: 2026-11-10T07:58:28Z
---
# Hook integration and smoke test

Prove the binary works as a real `commit-msg` hook, per spec §7.

Scope:
- A test that creates a temporary repository, installs `exec commit-msg-attribution-checker verify "$1"` as `.git/hooks/commit-msg` there, and drives real `git commit` calls. Use the built binary path, not a `PATH` lookup.
- The temporary repo sets `user.name` and `user.email` so identity targets resolve and the test does not depend on the developer config.
- Assert a blocked commit leaves no commit object: `git log` fails and `git rev-parse HEAD` fails.
- Assert `git commit --no-verify` proceeds with the same message, which records the soft-gate limitation from spec §2 rather than hiding it.
- Document the hook snippet in the README, since the harness ticket may only create `AGENTS.md` and a Makefile.

Done when:
- Acceptance criterion 13 passes against a real `git commit`.
- The temporary repo is deleted and the test never writes to the developer real hooks or the real config path.
- The documented snippet is byte-identical to the one the test installs.
- A test asserts the hook does not fire for `git commit --no-verify`, documenting the known gap.
