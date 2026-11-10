# commit-msg-attribution-checker

Reject a commit message that records LLM-agent attribution. Installed as a
`commit-msg` hook, the exit status aborts the commit.

## Table of contents

- [Motivation](#motivation)
- [Install as a hook](#install-as-a-hook)
- [Or register it with krok](#or-register-it-with-krok)

## Motivation

A `Made-with` or `Generated-by` line names the tool and the session that
produced a commit, not what the change does, so the noise grows while the
information does not. The class is not specific to AI: `Co-authored-by`
already credits an operating system or a hardware vendor. A match here means
a configured pattern matched, never that an agent wrote the code, and a
`Co-authored-by` naming a person passes.

## Install as a hook

Write this to `.git/hooks/commit-msg` and `chmod +x` it:

```sh
#!/bin/sh
exec commit-msg-attribution-checker verify "$1"
```

The message file comes in as `$1`, and a non-zero exit aborts the commit.

A `commit-msg` hook is a weak gate. These paths skip it: `git commit
--no-verify`, a foreign `core.hooksPath`, `git revert`, `git cherry-pick`,
`git am`, and plumbing commands. The same binary over a pushed range in CI
stays the authoritative check; `doc/spec.md` §2 covers the local hook path
only.

## Or register it with krok

[krok](https://github.com/yantonov/krok) is a git hook manager: it owns
`.git/hooks/<name>` with one small wrapper and runs its registered jobs in
order. Use it instead of writing the script above when the hook already
carries other jobs, or when you already run krok elsewhere.

```sh
krok add commit-msg commit-msg-attribution-checker verify
```

That installs the wrapper and records the job. krok keeps any hook script it
finds, moving it to `.git/krok/commit-msg/existing` and running it first, so
adding krok to a checkout that already has the plain script loses nothing. The
wrapper passes the message file to the tool the same way, so the exit status
still aborts the commit.

krok runs under the `sh` git ships, which on Windows means Git Bash.
