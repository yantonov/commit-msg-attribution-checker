# Filtering commits with explicit LLM-agent attribution

Research date: 2026-11-09.

## Scope

This guide covers commits that explicitly record AI or agent involvement in their message, author or committer identity, or attached attribution metadata. It does not attempt to infer AI usage from code style, commit wording, repository configuration files, or branch names.

The goal is to recognize the major attribution formats with broad coverage and explainable results. An exhaustive fixed list is not possible because attribution can be arbitrary user-defined text. “Almost all” should be measured against a labeled sample of the repositories being filtered.

Attribution usually follows the **harness or workflow**, not the model provider. A harness can use multiple providers, and a trailer can name a model without naming its harness.

## 1. Common attribution families

| Family | Example | Filtering rule |
| --- | --- | --- |
| Co-author trailer | `Co-authored-by: Claude … <noreply@anthropic.com>` | Require a recognized AI identity in the value. Human co-author trailers are common. |
| Assistance trailer | `Assisted-by: <tool/model>` | Classify the value; follow repository policy where available. |
| Tool trailer | `Made-with: Cursor` | Recognize the tool identity and spelling variants. |
| Free-form footer | `🤖 Generated with [OpenCode](https://opencode.ai)` | Match an attribution phrase plus a recognized tool, model, or URL. |
| Session provenance | `Amp-Thread-ID: https://ampcode.com/threads/…` | Recognize the key and validate the URL host/path. |
| Agent author or committer | `Cursor Agent <cursoragent@cursor.com>` | Match known agent identities in either metadata field. |
| Author-name decoration | `Human Name (aider)` | Recognize the harness-specific suffix. |
| Subject prefix | `aider: fix validation` | Treat as a candidate: the subject can also describe the tool itself. |
| Attached metadata | Git AI authorship log in `refs/notes/ai` | Parse the metadata and verify that it records AI attribution. |

GitHub documents `Co-authored-by: NAME <EMAIL>` as its co-author convention, including multiple co-authors. This syntax is not AI-specific. [1]

## 2. Documented harness conventions

| Harness or workflow | Signals to recognize | Qualification |
| --- | --- | --- |
| Claude Code | `Co-Authored-By: Claude … <noreply@anthropic.com>`; `Claude-Session: …` | The display name can contain the model/version, be `Claude`, or be `Claude Code`. Commit attribution and session URLs are configurable. [2] |
| Cursor IDE / CLI | “Made with Cursor” attribution; punctuation variants such as `Made-with: Cursor` | Official Git integration documentation describes the wording and an attribution setting. Do not assume one literal serialization across versions. [3] |
| Cursor cloud on GitHub | Author and committer `Cursor Agent <cursoragent@cursor.com>` | A co-author trailer can identify the human initiator. GitLab uses the project access token's bot user, requiring platform-specific identity mapping. [4] |
| Copilot CLI | `Co-authored-by: …` | `includeCoAuthoredBy` is documented and defaults to true. The configuration reference establishes the trailer behavior, not a universal identity for every Copilot surface. [5] |
| Copilot in VS Code | AI `Co-authored-by: …` trailer | Controlled by `git.addAICoAuthor`; the documented default is off. Applies to commits made through VS Code's Git integration. [6] |
| Copilot cloud | Copilot as primary author; human as co-author; session-log link | A filter examining only AI co-authors misses this arrangement. [7] |
| Amp | `Co-authored-by: Amp <amp@ampcode.com>`; `Amp-Thread-ID: <thread URL>`; Amp as primary author | Both trailers have documented controls. Hosted workflows can make Amp the author and the human the co-author. [8][9] |
| Aider | Co-author trailer; `(aider)` author/committer suffix; optional `aider:` subject prefix | Supports multiple attribution modes. Current options document co-author attribution as enabled by default; preserve older metadata patterns too. [10][11] |
| Repository-defined policy | `Assisted-by: <tool/model>` | Linux kernel submission documentation provides an example of this convention. Repository policy can override harness conventions. [12] |

### Historical, user-configured, or not established as universal defaults

| Harness | Potential signals | Evidence status |
| --- | --- | --- |
| OpenCode | `Generated with [opencode](https://opencode.ai)`; `Co-Authored-By: opencode <noreply@opencode.ai>` | Recorded in its issue tracker. Useful historical patterns, not a verified promise about every current version or plugin. [13] |
| Gemini CLI | Co-author values containing `Gemini`, `Gemini-CLI`, or a Gemini model | Its tracker records user-configured trailers and identity uncertainty. Do not treat those examples as one officially guaranteed identity. [14] |
| Codex | Explicit attribution values identifying `Codex`, `OpenAI Codex`, or a GPT model | A universal default trailer was not established from the official OpenAI documentation checked for this guide. These are candidate identity aliases, not a documented default assertion. [15] |
| Other harnesses and custom wrappers | Cline, Roo Code, Kilo Code, Goose, OpenHands, Devin, Factory Droid, Windsurf, and others | Extend the registry using verified documentation or observed explicit attribution. This guide does not assert default trailers or canonical emails for these tools. |

Do not interpret a missing documented convention as evidence that a tool never emits attribution.

## 3. Recommended filtering pipeline

### A. Collect raw fields

For each commit, retain:

- Commit SHA.
- Raw author name and email.
- Raw committer name and email.
- Full commit message, including subject and body.
- Resolved platform account identifiers, when available.
- Attached attribution notes, only if included in scope.

For analysis of original identities, use raw Git fields rather than only mailmap-normalized display values. A human merge or squash commit may have different metadata from the original agent commit.

### B. Parse structured trailers first

Use `git interpret-trailers --parse` or an equivalent Git-aware parser. Normalize keys case-insensitively and preserve repeated keys. [16]

Recognize attribution keys such as:

```text
Co-authored-by
Assisted-by
Generated-by
Made-with
AI-assisted-by
```

Only some of these are documented harness conventions above. The remainder provide permissive support for custom attribution.

Classify the **value**, not merely the presence of the key. For example:

```text
Co-authored-by: Alice Example <alice@example.com>
```

does not establish AI involvement.

### C. Fall back to line-oriented message matching

For malformed trailers, unusual capitalization, or attribution embedded in a squash message, use a fallback matcher.

The following regexes use Python/PCRE-style inline flags: `i` for case-insensitive and `m` for multiline. Port flags and Unicode handling to your regex engine.

Candidate attribution lines:

```regex
(?im)^[ \t]*(?:co[- ]?authored[- ]by|assisted[- ]by|generated[- ]by|made[- ]with|ai[- ]assisted[- ]by)[ \t]*:[ \t]*([^\r\n]+)
```

Candidate free-form footers:

```regex
(?im)^[ \t]*(?:🤖[ \t]*)?(?:generated|made|written|created|implemented|assisted)[ -]+(?:with|by|using)[ \t]*:?[ \t]+([^\r\n]+)
```

Known provenance keys:

```regex
(?im)^[ \t]*(?:claude-session|amp-thread-id)[ \t]*:[ \t]*(\S+)
```

Aider name decoration, applied to author and committer names:

```regex
(?i)\(aider\)[ \t]*$
```

Aider subject prefix, as a candidate rather than an unconditional match:

```regex
(?i)^aider[ \t]*:
```

These are extraction patterns, not complete classifiers. A candidate attribution line still needs identity validation. The provenance matcher needs URL validation.

An embedded line in a quoted example, reverted message, or discussion of attribution should receive lower confidence than an actual trailer.

### D. Classify the captured identity

Maintain a versioned registry with:

| Registry field | Purpose |
| --- | --- |
| Canonical tool ID | Stable internal identity, independent of display spelling. |
| Name aliases | Case-insensitive harness names and historical spellings. |
| Model aliases | Recognize model-only attribution without inventing a harness. |
| Exact emails | Prefer verified exact matches over broad domain matching. |
| Platform account IDs | Stable bot or agent account IDs where available. |
| Provenance keys and URL rules | Expected key, host, and path combinations. |
| Source and verification date | Distinguish documented, observed, and proposed patterns. |
| Match strength | Control automatic inclusion versus review. |

Examples of candidate aliases include `Claude`, `Claude Code`, `Codex`, `Copilot`, `Cursor`, `Amp`, `Aider`, `OpenCode`, and `Gemini CLI`.

Model-only attribution may name GPT, Gemini, DeepSeek, Qwen, or another model family. Recognize it in an attribution context. Do not classify a commit solely because those words occur somewhere in the message.

Avoid broad rules such as “any address at a provider domain is an agent.” Human employees use those domains too. Likewise, `[bot]` alone does not establish LLM involvement.

### E. Inspect author and committer separately

A known agent identity in either field is relevant, but preserve the role:

- Agent author: explicit agent authorship.
- Agent committer: explicit agent participation in committing.
- Human author plus AI co-author: explicit assistance/co-authorship.
- Agent author plus human co-author: explicit agent authorship with human credit.

These are different claims. In particular, Aider can commit preexisting human changes, so agent committer attribution does not necessarily mean the agent authored the code. [11]

A GitHub “Verified” signature badge alone is not AI attribution.

### F. Optionally inspect Git notes

Git AI defines authorship logs under `refs/notes/ai`, with structured attribution data. This is a separate channel from commit messages. [17]

If enabled, parse the log's schema and attribution records. The existence of a note alone should not establish AI-authored lines: the format also supports known-human attribution.

Notes may need separate fetching and may not be present in an ordinary local clone. Report whether this channel was inspected.

## 4. Filtering decisions and output

Use three outcomes:

| Outcome | Meaning | Suggested treatment |
| --- | --- | --- |
| Confirmed explicit attribution | Recognized AI attribution value, verified agent identity, or validated provenance marker | Include in the explicit-attribution set. |
| Candidate explicit attribution | Generic AI byline, unfamiliar identity, ambiguous name, malformed footer, or quoted marker | Review or include only in a broader candidate set. |
| No recognized explicit attribution | No supported marker found in inspected channels | Outside the matched set; no claim about actual AI use. |

“Confirmed” here means confirmed **recorded attribution**, not independently verified authorship. Git metadata and message text can be spoofed.

For each match, retain something like:

```json
{
  "commit": "<sha>",
  "classification": "confirmed_explicit_attribution",
  "tool": "amp",
  "model": null,
  "evidence": [
    {
      "field": "message.trailer",
      "key": "Co-authored-by",
      "value": "Amp <amp@ampcode.com>",
      "rule_id": "amp.coauthor"
    }
  ],
  "registry_version": "<version>"
}
```

Retain multiple matches: one commit can credit multiple tools or models. An attribution naming only a model should leave the harness unknown.

For a message-only filter, omit author/committer and notes rules explicitly. Otherwise the filter mixes distinct scopes.

## 5. Common false positives and coverage gaps

| Situation | Handling |
| --- | --- |
| Human co-author | Do not match the trailer key alone. |
| Commit about an AI product | Require attribution syntax or verified agent metadata. |
| Human named Claude, Devin, or another ambiguous alias | Require a stronger identity signal. |
| Generic automation bot | Require a verified LLM-agent identity. |
| Quoted trailer or documentation example in the message | Lower confidence or review context. |
| Squash message containing several original bylines | Preserve evidence; do not infer that every changed line came from the named agent. |
| Agent only generated the commit message | Attribution establishes recorded participation, not necessarily code generation. |
| Custom or localized attribution text | Add corpus-specific patterns; the English regexes are not language-exhaustive. |
| New model or harness names | Update the identity registry independently of the syntax matchers. |
| Custom company agent identity | Add repository/organization-specific mappings with evidence. |

Do not use `Signed-off-by` as a generic AI-attribution marker. It serves a different contribution-certification purpose; Linux kernel guidance explicitly reserves its DCO certification for humans. [18]

## 6. Suggested implementation policy

1. Automatically include valid attribution trailers naming known AI identities, verified agent metadata, and validated agent provenance links.
2. Keep free-form and custom-marker matches as a separate candidate channel until their identities and context are validated.
3. Preserve evidence and classification reasons, rather than returning only a boolean.
4. Version the rule registry and keep documented defaults separate from historical or user-configured examples.
5. Evaluate precision and recall against a labeled sample of **explicitly attributed commits**. Do not report recall over all AI usage.
6. Recheck conventions periodically because harness versions, deployment surfaces, and repository policies differ.

The reusable design is **attribution syntax → identity/provenance classification → evidence-backed filtering decision**.

## Sources

Sources below document specific conventions; they do not collectively define a universal standard.

1. [GitHub: Creating a commit with multiple authors](https://docs.github.com/en/pull-requests/how-tos/commit-changes/creating-a-commit-with-multiple-authors)
2. [Claude Code: All settings — attribution](https://code.claude.com/docs/en/settings-reference#attribution-commit)
3. [Cursor: Git integration](https://cursor.com/help/integrations/git)
4. [Cursor: Cloud agent secrets and network — commit author](https://cursor.com/docs/cloud-agent/security-network)
5. [GitHub: Copilot CLI configuration directory](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)
6. [VS Code: Staging and committing changes — AI co-author attribution](https://code.visualstudio.com/docs/sourcecontrol/staging-commits)
7. [GitHub: Managing agent sessions — trace commits to session logs](https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents)
8. [Amp: CLI configuration](https://ampcode.com/docs/cli/settings)
9. [Amp: GitHub and Git — commit identity](https://ampcode.com/docs/github)
10. [Aider: Options reference](https://aider.chat/docs/config/options)
11. [Aider: Git integration](https://aider.chat/docs/git.html)
12. [Linux kernel: Submitting patches — Assisted-by](https://docs.kernel.org/7.3/process/submitting-patches.html)
13. [OpenCode issue #558: Commit/PR footer configuration](https://github.com/anomalyco/opencode/issues/558) — historical behavior reported in the project's tracker.
14. [Gemini CLI issue #12419: Co-author attribution](https://github.com/google-gemini/gemini-cli/issues/12419) — user configuration and identity discussion.
15. [OpenAI: Codex configuration reference](https://developers.openai.com/codex/config-reference/) — checked without establishing a universal default attribution trailer.
16. [Git: git-interpret-trailers](https://git-scm.com/docs/git-interpret-trailers)
17. [Git AI: Authorship log specification v3.0.0](https://github.com/git-ai-project/git-ai/blob/main/specs/git_ai_standard_v3.0.0.md)
18. [Linux kernel: AI Coding Assistants](https://docs.kernel.org/next/process/coding-assistants.html)

