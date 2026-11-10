---
status: closed
priority: 2
deps: [match-engine]
attachments: []
created: 2026-11-10T06:39:04Z
updated: 2026-11-10T07:38:13Z
closed-at: 2026-11-10T07:38:13Z
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

Remaining (deferred to cmd-verify):
- Done-when bullet 1 is not closed here: criterion 12 needs `verify` to exit on the message result, and `verify` still exits 2 as not implemented. `cmd-verify` asserts it end to end once the output path exists.
- Resolution, rendering, the skip decision, the flag overrides, the missing-binary error and the committer-falls-back-to-author case are all implemented and unit-tested here.
- Noticed, not fixed (registry change, needs agreement): `aider.author-name` is anchored with `\(aider\)\s*$`, but spec §5.2 renders an identity as `Name <email>`, so the anchored pattern cannot match a real aider author read through `git var`. Pinned by tests/identity_targets.rs::the_aider_name_pattern_does_not_match_a_rendered_ident.
