//! The built-in pattern registry (spec §4).
//!
//! One list of records, and everything else derives from it: `config defaults`
//! and `config create` serialize it, and [`default_patterns`] parses that same
//! serialized text back into the list the resolver falls back to (spec §10).
//! A second hand-written copy of the patterns, or a TOML string literal, would
//! let the printed file and the shipped patterns drift apart.

// Reachable once `config defaults` and `config create` land; clippy runs with
// -D warnings, so the module stays quiet until then.
#![allow(dead_code)]

use serde::Serialize;

use crate::config::{Config, Target};

/// Emitted as `version = 1`. Informational in v1 (spec §3.4), but shown so a
/// created file matches the schema the spec documents.
const VERSION: u32 = 1;

/// One default pattern, in the shape the spec table prints it: `id`, `key`,
/// `regex`, `description`, plus the `target` the table columns imply.
#[derive(Debug, Clone, Copy, Serialize)]
struct DefaultPattern {
    id: &'static str,
    description: &'static str,
    target: Target,
    /// Only trailer patterns carry a key, because the config model rejects a
    /// key on any other target (spec §3.4). Skipping `None` keeps every
    /// emitted record loadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<&'static str>,
    /// Held as the spec's text rather than a compiled `Regex`: the config model
    /// compiles on load, and a compiled regex cannot be written back as the
    /// source the spec documents.
    regex: &'static str,
}

/// The document `config defaults` prints and `config create` writes.
#[derive(Serialize)]
struct DefaultsFile {
    version: u32,
    /// Serialized as a TOML array of tables, `[[pattern]]` (spec §3.4).
    #[serde(rename = "pattern")]
    pattern: &'static [DefaultPattern],
}

/// The registry in spec order, grouped by the spec's subsections so the
/// emitted file can be compared against the tables one at a time.
const DEFAULT_PATTERNS: &[DefaultPattern] = &[
    // §4.1 Attribution trailers that name a known AI identity.
    DefaultPattern {
        id: "co-authored-by.claude",
        description: "Co-authored-by trailer naming Claude",
        target: Target::Trailer,
        key: Some("co-authored-by"),
        regex: r"(?i)(\bclaude\b|@anthropic\.com)",
    },
    DefaultPattern {
        id: "co-authored-by.codex",
        description: "Co-authored-by trailer naming Codex or OpenAI",
        target: Target::Trailer,
        key: Some("co-authored-by"),
        regex: r"(?i)(\bcodex\b|\bopenai\b)",
    },
    DefaultPattern {
        id: "co-authored-by.copilot",
        description: "Co-authored-by trailer naming Copilot",
        target: Target::Trailer,
        key: Some("co-authored-by"),
        regex: r"(?i)\bcopilot\b",
    },
    DefaultPattern {
        id: "co-authored-by.cursor",
        description: "Co-authored-by trailer naming Cursor",
        target: Target::Trailer,
        key: Some("co-authored-by"),
        regex: r"(?i)(\bcursor\b|@cursor\.com)",
    },
    DefaultPattern {
        id: "co-authored-by.amp",
        description: "Co-authored-by trailer naming Amp",
        target: Target::Trailer,
        key: Some("co-authored-by"),
        regex: r"(?i)(\bamp\b|@ampcode\.com)",
    },
    DefaultPattern {
        id: "co-authored-by.aider",
        description: "Co-authored-by trailer naming Aider",
        target: Target::Trailer,
        key: Some("co-authored-by"),
        regex: r"(?i)\baider\b",
    },
    DefaultPattern {
        id: "co-authored-by.opencode",
        description: "Co-authored-by trailer naming OpenCode",
        target: Target::Trailer,
        key: Some("co-authored-by"),
        regex: r"(?i)(\bopencode\b|@opencode\.ai)",
    },
    DefaultPattern {
        id: "co-authored-by.gemini",
        description: "Co-authored-by trailer naming Gemini",
        target: Target::Trailer,
        key: Some("co-authored-by"),
        regex: r"(?i)\bgemini\b",
    },
    // §4.2 Trailers that carry only attribution: any non-empty value matches.
    DefaultPattern {
        id: "assisted-by",
        description: "Assisted-by trailer recording assistance",
        target: Target::Trailer,
        key: Some("assisted-by"),
        regex: r"\S",
    },
    DefaultPattern {
        id: "generated-by",
        description: "Generated-by trailer recording generation",
        target: Target::Trailer,
        key: Some("generated-by"),
        regex: r"\S",
    },
    DefaultPattern {
        id: "ai-assisted-by",
        description: "AI-assisted-by trailer recording AI assistance",
        target: Target::Trailer,
        key: Some("ai-assisted-by"),
        regex: r"\S",
    },
    DefaultPattern {
        id: "made-with",
        description: "Made-with trailer naming a tool",
        target: Target::Trailer,
        key: Some("made-with"),
        regex: r"\S",
    },
    // §4.3 Provenance keys.
    DefaultPattern {
        id: "claude-session",
        description: "Claude session reference",
        target: Target::Trailer,
        key: Some("claude-session"),
        regex: r"\S",
    },
    DefaultPattern {
        id: "amp-thread-id",
        description: "Provenance link to an Amp thread",
        target: Target::Trailer,
        key: Some("amp-thread-id"),
        regex: r"^https?://ampcode\.com/",
    },
    // §4.4 Free-form footer: three records, because each carries its own
    // second signal (emoji, a known tool or model, a URL).
    DefaultPattern {
        id: "free-form-footer.emoji",
        description: "Robot-marked attribution footer",
        target: Target::Line,
        key: None,
        regex: r"(?i)^\s*🤖\s*(?:generated|made|written|created|implemented|assisted)[ -]+(?:with|by|using)",
    },
    DefaultPattern {
        id: "free-form-footer.tool",
        description: "Attribution footer naming a known tool or model",
        target: Target::Line,
        key: None,
        regex: r"(?i)^\s*(?:generated|made|written|created|implemented|assisted)[ -]+(?:with|by|using)[: ]+.*(?:\bclaude\b|\bcodex\b|\bcopilot\b|\bcursor\b|\bamp\b|\baider\b|\bopencode\b|\bgemini\b|\bgpt\b|\bopenai\b|\bdeepseek\b|\bqwen\b)",
    },
    DefaultPattern {
        id: "free-form-footer.url",
        description: "Attribution footer carrying a URL",
        target: Target::Line,
        key: None,
        regex: r"(?i)^\s*(?:generated|made|written|created|implemented|assisted)[ -]+(?:with|by|using)[: ]+.*https?://",
    },
    // §4.5 Aider decoration.
    DefaultPattern {
        id: "aider.author-name",
        description: "Aider author-name decoration",
        target: Target::Author,
        key: None,
        // Not anchored: spec §5.2 renders an ident as `Name <email>`, so an
        // end anchor would never land on the decoration and the pattern could
        // not fire on a real ident.
        regex: r"(?i)\(aider\)",
    },
    DefaultPattern {
        id: "aider.committer-name",
        description: "Aider committer-name decoration",
        target: Target::Committer,
        key: None,
        regex: r"(?i)\(aider\)",
    },
    DefaultPattern {
        id: "aider.subject",
        description: "Aider subject prefix",
        target: Target::Subject,
        key: None,
        regex: r"(?i)^aider\s*:",
    },
    // §4.6 Agent identity in author or committer.
    DefaultPattern {
        id: "author.agent-identity",
        description: "Agent email identity as author",
        target: Target::Author,
        key: None,
        regex: r"(?i)(@cursor\.com|@ampcode\.com|@opencode\.ai)",
    },
    DefaultPattern {
        id: "committer.agent-identity",
        description: "Agent email identity as committer",
        target: Target::Committer,
        key: None,
        regex: r"(?i)(@cursor\.com|@ampcode\.com|@opencode\.ai)",
    },
];

/// The built-in patterns as TOML, byte for byte what `config defaults` prints
/// and `config create` writes.
///
/// Serialization cannot fail here: the document is a fixed record list whose
/// fields are strings, an integer and a unit enum, which TOML represents
/// without loss.
pub fn defaults_toml() -> String {
    toml::to_string(&DefaultsFile {
        version: VERSION,
        pattern: DEFAULT_PATTERNS,
    })
    .expect("the default registry serializes to TOML")
}

/// The built-in patterns as the resolver's fallback (spec §3.2 step 4).
///
/// Built by parsing the emitted TOML, so the running list and the file a user
/// receives are the same records, and a record that would not survive the
/// round trip fails here instead of on a user's machine.
pub fn default_patterns() -> Config {
    Config::parse(&defaults_toml()).expect("the default registry round-trips through Config::parse")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The id set, in registry order: spec §4.1 to §4.6, table by table.
    const EXPECTED_IDS: &[&str] = &[
        "co-authored-by.claude",
        "co-authored-by.codex",
        "co-authored-by.copilot",
        "co-authored-by.cursor",
        "co-authored-by.amp",
        "co-authored-by.aider",
        "co-authored-by.opencode",
        "co-authored-by.gemini",
        "assisted-by",
        "generated-by",
        "ai-assisted-by",
        "made-with",
        "claude-session",
        "amp-thread-id",
        "free-form-footer.emoji",
        "free-form-footer.tool",
        "free-form-footer.url",
        "aider.author-name",
        "aider.committer-name",
        "aider.subject",
        "author.agent-identity",
        "committer.agent-identity",
    ];

    fn ids(config: &Config) -> Vec<&str> {
        config
            .patterns
            .iter()
            .map(|pattern| pattern.id.as_str())
            .collect()
    }

    fn pattern<'a>(config: &'a Config, id: &str) -> &'a crate::config::Pattern {
        config
            .patterns
            .iter()
            .find(|pattern| pattern.id == id)
            .unwrap_or_else(|| panic!("the registry holds {id}"))
    }

    #[test]
    fn registry_is_exactly_the_spec_id_set_in_order() {
        // Exact equality rather than a subset check: an added or dropped
        // pattern changes the shipped behaviour, so it must break this test.
        assert_eq!(DEFAULT_PATTERNS.len(), 22);
        assert_eq!(EXPECTED_IDS.len(), 22);
        assert_eq!(
            DEFAULT_PATTERNS
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>(),
            EXPECTED_IDS
        );

        let config = default_patterns();
        assert_eq!(ids(&config), EXPECTED_IDS);
    }

    #[test]
    fn every_pattern_has_a_description() {
        // spec §4: the description prints in the rejection message, so an empty
        // one leaves the user without a reason.
        for record in DEFAULT_PATTERNS {
            assert!(
                !record.description.trim().is_empty(),
                "{}: empty description",
                record.id
            );
        }

        for pattern in &default_patterns().patterns {
            assert!(
                !pattern.description.trim().is_empty(),
                "{}: empty description after the round trip",
                pattern.id
            );
        }
    }

    #[test]
    fn free_form_footer_is_three_records() {
        // spec §4.4: one record per second signal, not one record for the
        // phrase, so a report names why the line matched.
        let footer: Vec<&str> = DEFAULT_PATTERNS
            .iter()
            .map(|record| record.id)
            .filter(|id| id.starts_with("free-form-footer."))
            .collect();

        assert_eq!(
            footer,
            [
                "free-form-footer.emoji",
                "free-form-footer.tool",
                "free-form-footer.url"
            ]
        );

        let config = default_patterns();
        for id in footer {
            let pattern = pattern(&config, id);
            assert!(
                pattern.key.is_none() && pattern.target == Target::Line,
                "{id}: a footer pattern tests the whole line and takes no key"
            );
        }
    }

    #[test]
    fn bare_word_identities_still_match() {
        // spec §4.1: Aider, Amp and Gemini publish no domain to anchor on, so
        // the tool over-matches on the bare word on purpose. Those values carry
        // no email, and must still match.
        let config = default_patterns();
        for (id, value) in [
            ("co-authored-by.aider", "Aider"),
            ("co-authored-by.amp", "Amp"),
            ("co-authored-by.gemini", "Gemini"),
        ] {
            let pattern = pattern(&config, id);
            assert!(
                pattern.regex.is_match(value),
                "{id} must match the bare word {value:?}, got {:?}",
                pattern.regex.as_str()
            );
            assert!(
                pattern.regex.is_match(&value.to_lowercase()),
                "{id} matches case-insensitively"
            );
        }
    }

    #[test]
    fn emitted_toml_round_trips_without_changing_a_field() {
        // The emitted text is what a user edits, so a field mangled by
        // serialization (backslashes, `(?i)`, brackets) would silently change
        // matching. Compare every field against the record it came from.
        let text = defaults_toml();
        let config = Config::parse(&text)
            .unwrap_or_else(|err| panic!("emitted defaults load: {err}\n{text}"));

        assert_eq!(config.patterns.len(), DEFAULT_PATTERNS.len());
        for (record, loaded) in DEFAULT_PATTERNS.iter().zip(&config.patterns) {
            assert_eq!(loaded.id, record.id);
            assert_eq!(loaded.description, record.description, "{}", record.id);
            assert_eq!(loaded.target, record.target, "{}", record.id);
            assert_eq!(
                loaded.key.as_deref(),
                record.key,
                "{}: trailer keys survive",
                record.id
            );
            assert_eq!(
                loaded.regex.as_str(),
                record.regex,
                "{}: the emitted regex text is the registry text",
                record.id
            );
        }
    }

    #[test]
    fn emitted_toml_declares_the_version_and_one_table_per_pattern() {
        let text = defaults_toml();

        assert!(
            text.starts_with("version = 1\n"),
            "the schema example in spec §3.4 leads with version = 1:\n{text}"
        );
        assert_eq!(
            text.matches("[[pattern]]").count(),
            DEFAULT_PATTERNS.len(),
            "one TOML table per record"
        );
    }

    #[test]
    fn every_trailer_pattern_has_a_key_and_no_other_pattern_does() {
        // The config model enforces this pairing on load (spec §3.4); asserting
        // it here names the offending record instead of a parse error.
        for record in DEFAULT_PATTERNS {
            assert_eq!(
                record.target == Target::Trailer,
                record.key.is_some(),
                "{}: the key belongs to trailer patterns only",
                record.id
            );
        }
    }

    #[test]
    fn defaults_are_stable_across_builds() {
        // `config create` followed by a second `config create` must find the
        // same bytes, and a test fixture elsewhere compares against this text.
        assert_eq!(defaults_toml(), defaults_toml());
        assert_eq!(ids(&default_patterns()), ids(&default_patterns()));
    }
}
