# commit-msg-attribution-checker — specification

Status: draft (v1).

Companion research: [`llm_commit_attribution_filtering.md`](llm_commit_attribution_filtering.md).

## 1. Purpose

Reject a commit whose message records LLM-agent attribution. The main call
site is the `commit-msg` hook. The tool runs before git writes the commit
object, so its exit status can abort the commit.

The tool answers one question: does this message contain a marker that the
configured pattern list rejects? It does not infer AI use from code, from
commit wording without a marker, from branch names, or from repository
configuration.

## 2. Non-goals

- **No project-level configuration.** v1 reads one user config file. A
  repository cannot add, weaken, or remove patterns.
- **No `refs/notes/ai` (Git AI) channel.** Git attaches notes to a commit that
  already exists. A `commit-msg` hook cannot see them. The channel is out of
  scope, not silently unsupported.
- **No platform-side identities.** A local hook never sees GitHub or Copilot
  bot accounts, GitLab bot users, or a squash merge that a server produces.
- **No inference.** No match means "the configured patterns matched nothing".
  It never means "the author used no AI".
- **No bypass mode.** The tool reports matches. It has no "allow once" option.

A `commit-msg` hook is a weak gate. These paths skip it: `git commit
--no-verify`, a foreign `core.hooksPath`, `git revert`, `git cherry-pick`,
`git am`, and plumbing commands. The same binary over a pushed range in CI
stays the authoritative check. This spec covers the local hook path only.

## 3. Configuration

### 3.1 Location

| Item | Value |
| --- | --- |
| Config path | `$XDG_CONFIG_HOME/commit-msg-attribution-checker/config.toml` |
| Fallback | `~/.config/commit-msg-attribution-checker/config.toml`, when `XDG_CONFIG_HOME` is unset |
| Path override | `--config <path>` flag first, then `COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG` |
| Format | TOML |

The binary and the config file carry the full tool name. A user runs neither
often enough for a short alias to pay off, and the full name explains itself
in a hook script.

### 3.2 Resolution

The tool reads the config in this order:

1. If the user passes `--config <path>`, read that file. A missing or
   malformed file is an error (exit 2). The tool does not fall back to the
   defaults.
2. Else, if the user sets `COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG`, read that
   file. The same error rule applies.
3. Else, if the config file exists at the default path, read it.
4. Else, use the built-in defaults compiled into the binary.

### 3.3 User config replaces defaults

A user config file is the complete pattern list. The tool does not merge it
with the built-in defaults.

The tool intends each consequence below:

- A user file that lists one pattern disables every built-in pattern that it
  omits.
- A valid file with no `[[pattern]]` entries disables all checking.
- `config create` writes the full default set, so the natural starting point
  for an edit is a complete list.

### 3.4 Schema

```toml
# Optional. Shown by `config create`; not used for matching.
version = 1

[[pattern]]
id = "co-authored-by.claude"
description = "Co-authored-by trailer naming Claude"
target = "trailer"
key = "co-authored-by"
regex = '(?i)(\bclaude\b|@anthropic\.com)'
```

| Field | Required | Meaning |
| --- | --- | --- |
| `id` | yes | Stable identifier, unique within the file. The tool reports it. |
| `description` | yes | One sentence that states what the pattern rejects. The tool reports it. |
| `target` | yes | One of `trailer`, `line`, `subject`, `author`, `committer`. See §5. |
| `key` | only for `trailer` | Trailer key. The tool compares it without case. |
| `regex` | yes | Rust `regex` syntax. The tool tests it against the field value. |

Every pattern blocks. There is no severity field, because the contract of the
tool is "a match rejects the commit". There is no `enabled` field, because
replacement semantics mean the file lists exactly the live patterns. A user
who wants to disable a pattern removes the entry or comments it out.

`version` is informational in v1. An unknown value does not abort.

An unknown field is a config error (exit 2). A typo cannot silently disable a
rule.

## 4. Default patterns

These are the built-in defaults named in §3.2. `config create` writes them.
The identity lists follow the research doc: documented harness conventions
first, then model aliases.

The `description` strings below are the ones the tool ships. Keep them short,
because they print in the rejection message.

### 4.1 Attribution trailers that name a known AI identity

| id | key | regex | description |
| --- | --- | --- | --- |
| `co-authored-by.claude` | `co-authored-by` | `(?i)(\bclaude\b\|@anthropic\.com)` | Co-authored-by trailer naming Claude |
| `co-authored-by.codex` | `co-authored-by` | `(?i)(\bcodex\b\|\bopenai\b)` | Co-authored-by trailer naming Codex or OpenAI |
| `co-authored-by.copilot` | `co-authored-by` | `(?i)\bcopilot\b` | Co-authored-by trailer naming Copilot |
| `co-authored-by.cursor` | `co-authored-by` | `(?i)(\bcursor\b\|@cursor\.com)` | Co-authored-by trailer naming Cursor |
| `co-authored-by.amp` | `co-authored-by` | `(?i)(\bamp\b\|@ampcode\.com)` | Co-authored-by trailer naming Amp |
| `co-authored-by.aider` | `co-authored-by` | `(?i)\baider\b` | Co-authored-by trailer naming Aider |
| `co-authored-by.opencode` | `co-authored-by` | `(?i)(\bopencode\b\|@opencode\.ai)` | Co-authored-by trailer naming OpenCode |
| `co-authored-by.gemini` | `co-authored-by` | `(?i)\bgemini\b` | Co-authored-by trailer naming Gemini |

The key alone never matches. `Co-authored-by: Alice <alice@example.com>` names
a legitimate human co-author, so it passes.

The bare-word matches (`aider`, `amp`, `gemini`) are the weak point of this
set. They also catch a human with that name. Aider, Amp, and Gemini give no
domain to anchor on, so the tool over-matches on purpose: a missed attribution
is worse than a rejected commit that the user can reword or dispute. Aliases
with a domain (`@anthropic.com`, `@cursor.com`, `@ampcode.com`,
`@opencode.ai`) match by domain and by name.

### 4.2 Trailers that carry only attribution

These keys exist only to record assistance or a tool. Any non-empty value
matches.

| id | key | regex | description |
| --- | --- | --- | --- |
| `assisted-by` | `assisted-by` | `\S` | Assisted-by trailer recording assistance |
| `generated-by` | `generated-by` | `\S` | Generated-by trailer recording generation |
| `ai-assisted-by` | `ai-assisted-by` | `\S` | AI-assisted-by trailer recording AI assistance |
| `made-with` | `made-with` | `\S` | Made-with trailer naming a tool |

### 4.3 Provenance keys

| id | key | regex | description |
| --- | --- | --- | --- |
| `claude-session` | `claude-session` | `\S` | Claude session reference |
| `amp-thread-id` | `amp-thread-id` | `^https?://ampcode\.com/` | Provenance link to an Amp thread |

The `amp-thread-id` pattern checks the host. It does not accept any value, so
a user can keep the key for a reference that carries no attribution.

### 4.4 Free-form footer

A footer line matches only when a second signal accompanies the attribution
phrase: the `🤖` marker, a known tool or model name, or a URL. The phrase
alone is not enough.

| id | target | regex | description |
| --- | --- | --- | --- |
| `free-form-footer.emoji` | `line` | `(?i)^\s*🤖\s*(?:generated\|made\|written\|created\|implemented\|assisted)[ -]+(?:with\|by\|using)` | Robot-marked attribution footer |
| `free-form-footer.tool` | `line` | `(?i)^\s*(?:generated\|made\|written\|created\|implemented\|assisted)[ -]+(?:with\|by\|using)[: ]+.*(?:\bclaude\b\|\bcodex\b\|\bcopilot\b\|\bcursor\b\|\bamp\b\|\baider\b\|\bopencode\b\|\bgemini\b\|\bgpt\b\|\bopenai\b\|\bdeepseek\b\|\bqwen\b)` | Attribution footer naming a known tool or model |
| `free-form-footer.url` | `line` | `(?i)^\s*(?:generated\|made\|written\|created\|implemented\|assisted)[ -]+(?:with\|by\|using)[: ]+.*https?://` | Attribution footer carrying a URL |

Do not match the phrase alone. These lines are ordinary prose about a build or
a human: `generated by the build script`, `Created by hand`, `written by the
release team`, `Made by Nix`, `generated with protoc`, and `implemented using
a visitor pattern`. The tool rejects a commit that carries no attribution when
it matches them. Under replacement semantics the user then pays with a config
edit or a reworded message.

The three patterns accept these lines:

| Line | Matches |
| --- | --- |
| `🤖 Generated with [OpenCode](https://opencode.ai)` | emoji, tool, url |
| `🤖 Generated with Amp` | emoji |
| `🤖 Generated with something unknown` | emoji |
| `Generated with Claude Code` | tool |
| `Made with Cursor` | tool |
| `Generated by GPT-5` | tool |
| `generated by https://example.com/gen` | url |

The tool misses a footer that names no tool, no model, and no URL, such as
`Generated with an LLM`. This gap is intentional. A user who wants the wider
net adds one pattern with the phrase alone. That one-line edit makes the
trade-off theirs, not everyone's.

### 4.5 Aider decoration

| id | target | regex | description |
| --- | --- | --- | --- |
| `aider.author-name` | `author` | `(?i)\(aider\)\s*$` | Aider author-name decoration |
| `aider.committer-name` | `committer` | `(?i)\(aider\)\s*$` | Aider committer-name decoration |
| `aider.subject` | `subject` | `(?i)^aider\s*:` | Aider subject prefix |

### 4.6 Agent identity in author or committer

| id | target | regex | description |
| --- | --- | --- | --- |
| `author.agent-identity` | `author` | `(?i)(@cursor\.com\|@ampcode\.com\|@opencode\.ai)` | Agent email identity as author |
| `committer.agent-identity` | `committer` | `(?i)(@cursor\.com\|@ampcode\.com\|@opencode\.ai)` | Agent email identity as committer |

The tool matches an identity pattern against `Name <email>`, after trimming,
with one space before the angle bracket.

## 5. Matching

### 5.1 Comment lines

The tool ignores a message line whose first byte is `#` before it matches
anything. This rule covers the comment lines that an interactive editor leaves
in `COMMIT_EDITMSG`.

Only the first byte counts. `  # Co-authored-by: …` is not a comment line, so
the tool matches it. This is git's own rule: `cleanup=strip` removes only a
`#` in the first column. The tool and the stored message therefore agree.

Not being a comment line is not the same as being claimed. Such a line is
available to the `line` and `subject` patterns, and to the trailer patterns
through the expression in §5.2, which is anchored at the start of the line: an
indented trailer has no trailer shape, so no trailer pattern tests it. The line
is clean unless a `line` or `subject` pattern matches it.

### 5.2 Targets

| target | Value the tool tests | Notes |
| --- | --- | --- |
| `trailer` | the value of every line that matches `^([A-Za-z0-9][A-Za-z0-9-]*):[ \t]*(.+)$`, when the key equals `key` without case | Scans the whole message, not only the last paragraph, so it catches attribution inside a squash message. |
| `line` | each non-comment line, as written, without the trailing newline | Skipped for a line that a trailer pattern already matched. |
| `subject` | the first non-comment line | The same skip applies, independently of the `line` skip. |
| `author` | the name and email from `git var GIT_AUTHOR_IDENT`, rendered as `Name <email>` | |
| `committer` | the name and email from `git var GIT_COMMITTER_IDENT`, same rendering | |

The tool implements the trailer parser. v1 does not invoke `git
interpret-trailers`, so matching works outside a repository and needs no git
binary.

Trailer patterns run first on every line. Line and subject patterns then run
on that line unless a trailer pattern already matched it. The gate is a
trailer match, not trailer shape. `aider: fix validation` has trailer shape,
but no configured trailer pattern claims it, so subject patterns still see it
and `aider.subject` matches. The rule stops `Assisted-by: Amp
<amp@ampcode.com>` from reporting twice, and it applies to exactly the lines
that a trailer pattern claimed.

The tool reads identity from `git var`. That command reflects
`GIT_AUTHOR_*`, `GIT_COMMITTER_*`, and the config fallback. Git leaves
`GIT_COMMITTER_*` unset when the committer defaults to the author, so the tool
must not read the environment variables directly. `git var` is the authority.
Outside a repository, the tool skips identity targets and reports the skip on
stderr at `-v`. The `--author` and `--committer` flags override the resolved
value.

### 5.3 Report of a match

The tool records the first match per pattern. A commit message that credits
three tools produces three matches, and the tool reports all of them before it
rejects the commit.

Patterns overlap across targets. `Made-with: Cursor` belongs to `made-with`
and also reads as a footer, but the trailer gate reports it once. The count in
the summary is a match count.

Each match records the pattern `id`, its `description`, the `target`, the
matched value, and the message line number where the match applies. Line
numbers start at 1 and count every line of the file, including comment lines.

### 5.4 Output

On a clean message the tool writes nothing and exits 0. Silence suits the
hook.

On a match the tool writes a block to stdout and exits 1.

```text
commit-msg-attribution-checker: attribution detected

  co-authored-by.claude (line 4, trailer co-authored-by)
    Co-authored-by trailer naming Claude
    value: Claude <noreply@anthropic.com>

  amp-thread-id (line 5, trailer amp-thread-id)
    Provenance link to an Amp thread
    value: https://ampcode.com/threads/T-019

2 matches. Commit rejected.
```

`--json` on `verify` writes the matches as JSON for CI. Exit codes do not
change.

The tool writes config and usage errors to stderr and exits 2.

## 6. Commands

Binary: `commit-msg-attribution-checker`.

| Command | Behaviour |
| --- | --- |
| `verify <msg-file>` | Check the message file. Flags: `--author <ident>`, `--committer <ident>`, `--json`, `-v`. |
| `config` | With no subcommand, write the subcommand list to stderr and exit 2. Same output as `config --help`. |
| `config defaults` | Write the built-in default config as TOML to stdout. |
| `config create` | Write the built-in default config to the resolved path, create the parent directories, and print the absolute path. Fails when the file exists. |
| `config show` | Write the effective config to stdout: the file in use, or the defaults. A header comment names the source path. |

`config create` never overwrites. When the target exists, the command fails
with exit 2, prints the path, and leaves the file unchanged. The user resolves
the conflict: edit the file, move it, or remove it. There is no `--force`.

`config show` shows the user what `verify` will apply. Under replacement
semantics the effective config differs from `config defaults` exactly when a
user file exists, and that is the case worth inspecting.

`--config <path>` is a global flag. Every command accepts it.

Exit codes: `0` clean, `1` attribution found, `2` usage, I/O, or config error.

## 7. Hook integration

Install the binary as `.git/hooks/commit-msg`:

```sh
#!/bin/sh
exec commit-msg-attribution-checker verify "$1"
```

`commit-msg` receives the message file path as `$1`, which is the `verify`
positional argument. A non-zero exit aborts the commit, and git shows the
output to the user.

The hook runs for `commit`, `commit --amend`, `merge`, and interactive rebase
steps that edit a message. It does not run for plain `rebase`, `revert
--no-edit`, `cherry-pick` without an editor, or `git am`.

## 8. Acceptance criteria

1. `verify` on a message with `Co-authored-by: Claude <noreply@anthropic.com>`
   exits 1 and names `co-authored-by.claude`.
2. `verify` on a message with `Co-authored-by: Alice <alice@example.com>`
   exits 0.
3. A message with three attribution lines reports three matches and exits 1.
4. A message whose first line is `# Co-authored-by: Claude …` exits 0. The same
   text with two leading spaces also exits 0: §5.1 keeps the line, but §5.2's
   trailer expression is anchored at the line start, so an indented trailer has
   no trailer shape and no shipped pattern claims it.
5. A body line `generated by the build script` exits 0. `Made with Cursor`
   exits 1. `🤖 Generated with something unknown` exits 1.
6. `Assisted-by: Amp <amp@ampcode.com>` reports one match, not two. The
   trailer gate keeps the footer patterns off a line that a trailer pattern
   already matched.
7. A subject `aider: fix validation` exits 1 and names `aider.subject`, even
   though the line has trailer shape.
8. An empty message file, or a file with comment lines only, exits 0.
9. `config create` followed by `verify` on a matching message exits 1. The
   written file gives the same behaviour.
10. A second `config create` fails with exit 2, prints the path, and leaves the
    existing file byte-identical.
11. A user config with one unrelated pattern lets a Claude trailer pass. This
    demonstrates replacement semantics.
12. `verify` outside a repository skips identity targets and does not fail.
13. A `commit-msg` hook that calls the binary blocks `git commit` and allows
    `git commit --no-verify`.

## 9. Deferred

- Second entry point over a pushed commit range, and the CI invocation that
  goes with it.
- `refs/notes/ai` inspection.
- Corpus-specific and localized patterns.
- Registry versioning in the output (the `registry_version` field in the
  research doc's evidence record).

## 10. Implementation

Plain `cargo`, edition 2024. No workspace, no build wrapper.

| Crate | Purpose |
| --- | --- |
| `clap` (derive) | Command and flag parsing |
| `regex` | Pattern matching |
| `serde` (derive) | Config deserialization |
| `toml` | Config parsing and serialization |
| `dirs` | `$XDG_CONFIG_HOME` and home resolution |

The defaults live in one Rust source file as a constant list of pattern
records. The same list is what `config defaults` prints and `config create`
writes, so the shipped patterns and their descriptions cannot diverge.
