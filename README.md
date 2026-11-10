CLI tool to validate commits

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
