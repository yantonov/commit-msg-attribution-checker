---
status: open
priority: 2
deps: [match-engine]
attachments: []
created: 2026-11-10T06:39:04Z
updated: 2026-11-10T06:39:11Z
---
# Identity targets via git var

Support the `author` and `committer` patterns, per spec §5.2.

Scope:
- Resolve identity with `git var GIT_AUTHOR_IDENT` and `git var GIT_COMMITTER_IDENT`, rendered as `Name <email>` after trimming. Do not read `GIT_AUTHOR_*` or `GIT_COMMITTER_*` directly: git leaves `GIT_COMMITTER_*` unset when the committer defaults to the author.
- `--author` and `--committer` flags override the resolved value.
- Outside a repository, skip identity targets, do not fail, and report the skip on stderr at `-v`.

Done when:
- Acceptance criterion 12 passes: outside a repo, `verify` skips identity targets and exits by message result.
- Tests cover: `git var` value used, flag override wins, `(aider)` suffix in the author name matches, agent email matches.
- A test asserts the tool does not read the env vars directly (committer-defaults-to-author case).
