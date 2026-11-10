---
status: open
priority: 3
deps: [cmd-verify]
attachments: []
created: 2026-11-10T06:39:08Z
updated: 2026-11-10T06:39:11Z
---
# Hook integration and smoke test

Prove the binary works as a real `commit-msg` hook, per spec §7.

Scope:
- A script or test that installs `exec commit-msg-attribution-checker verify "$1"` as `.git/hooks/commit-msg` in a temporary repo.
- A test that a blocked commit leaves no commit object, and that `git commit --no-verify` proceeds.
- Document the hook snippet in `AGENTS.md` or the README, whichever the harness ticket chose.

Done when:
- Acceptance criterion 13 passes against a real `git commit`.
- The temporary repo is cleaned up and the test does not touch the developer's real hooks.
- The documented snippet matches the one the test installs.
