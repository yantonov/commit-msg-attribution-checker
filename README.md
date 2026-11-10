CLI tool to validate commits

## Motivation

Attribution trailers and footers name the tool that produced a change. They do
not describe the change. A reader of history has to filter them out to reach
the subject, the reasoning and the review context that a commit exists to
carry.

The noise grows while the information does not. Attribution is per tool and
per session, so it accumulates with every agent, editor and harness in use,
and two commits with identical content read differently only because of which
tool touched them.

Nor is the class specific to AI. Co-author and made-with lines already credit
operating systems, hardware vendors and other tools, because they record who
or what was involved in producing the commit. That is metadata about the
making, not about the merit or the content of the change.

So this tool rejects the trailers that exist only to record making: a
`Made-with`, `Generated-by` or `Assisted-by` line blocks whatever it names,
and a `Co-authored-by` blocks when it names a known agent identity. It makes
no claim about how the code was written: a match means a configured pattern
matched, never that an agent authored the change, and a `Co-authored-by`
naming a person passes.

## Table of contents

- [Motivation](#motivation)
- [Install as a hook](#install-as-a-hook)
- [Or register it with krok](#or-register-it-with-krok)

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
