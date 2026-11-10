---
status: closed
priority: 2
deps: []
attachments: []
created: 2026-11-10T08:21:21Z
updated: 2026-11-10T08:22:47Z
closed-at: 2026-11-10T08:22:47Z
---
# Document installing the hook through krok

README gains a short optional section on registering the checker as a commit-msg job with krok (https://github.com/yantonov/krok), plus a link to the repo for information.

Scope:
- One short section under the existing hook install, not a replacement for it. The plain-script install stays the default; krok is offered for users who already run several jobs on a hook or already use krok.
- The krok route is one command: `krok add commit-msg commit-msg-attribution-checker verify`, which writes the wrapper at .git/hooks/commit-msg and records the job in .git/krok-config.yml.
- State the behaviour a user will actually see: krok preserves an existing hook script at .git/krok/commit-msg/existing and keeps running it, so adding krok to a checkout that already has the plain script does not lose that script. Verified against krok 0.3.0.
- Note that the wrapper execs `krok run commit-msg "$@"`, so the message file still arrives as $1 and the exit status still aborts the commit.
- Link https://github.com/yantonov/krok where the section introduces krok. Do not copy krok docs beyond the one command and the two behaviours above.

Done when:
- A commit carrying a Claude trailer is blocked and a clean commit succeeds with the hook installed by krok, asserted by a test, not only by hand.
- The README section names the command, the preserved-script behaviour and the repo link.
- The plain-script section is unchanged, and make check stays green.

Decision (human): docs-only, no test. The krok route is documented and was verified by hand against krok 0.3.0 (attributed commit blocked, clean commit succeeds, existing script preserved at .git/krok/commit-msg/existing). No automated test, so the suite keeps needing only git and no krok on the machine. README also gains a table of contents and the krok material as its own section.
