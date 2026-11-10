---
status: open
priority: 2
deps: [match-engine]
attachments: []
created: 2026-11-10T06:39:04Z
updated: 2026-11-10T06:40:19Z
---
# Identity targets via git var

Support the `author` and `committer` patterns, per spec §5.2.

Scope:
- Resolve identity with `git var GIT_AUTHOR_IDENT` and `git var GIT_COMMITTER_IDENT`, rendered as `Name <email>` after trimming. Do not read `GIT_AUTHOR_*` or `GIT_COMMITTER_*` directly: git leaves `GIT_COMMITTER_*` unset when the committer defaults to the author, so the env vars disagree with what git will record.
- `--author` and `--committer` flags override the resolved value. The flag value is used as given, so a caller can test a pattern without a repo.
- The tool shells out to `git var`, so the absence of a `git` binary is an I/O error, exit 2, not a silent skip. Only "not inside a repository" is a skip.
- Outside a repository, skip identity targets, do not fail, and report the skip on stderr at `-v`. Exit status then depends on the message targets alone.

Done when:
- Acceptance criterion 12 passes: outside a repo, `verify` skips identity targets and exits by the message result.
- Tests cover: `git var` value used when inside a repo with a real committer, flag override wins over both, `(aider)` suffix in the author name matches, agent email in the committer matches.
- A test covers the committer-defaults-to-author case and asserts the rendered committer equals the author, which the env-var approach would also get wrong in a different way.
- A test asserts a missing `git` binary exits 2 rather than skipping.
