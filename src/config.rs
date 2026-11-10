//! Typed representation of the config file (spec §3.4).
//!
//! One load path: TOML text in, validated patterns with compiled regexes out.
//! Nothing here reads a file or resolves a path — that is the
//! `config-resolution` ticket — so the whole model is a pure function and no
//! error can exit the process.

// `ConfigError::exit_code` stays unreferenced until `verify` prints a config
// error itself: every command that loads a config reaches it through
// `LoadError`, which carries the status. clippy runs with -D warnings.
#![allow(dead_code)]

use std::collections::HashSet;
use std::fmt;

use regex::Regex;
use serde::{Deserialize, Serialize};

/// The value a pattern is tested against (spec §5.2).
///
/// `Serialize` exists for the default registry: it renders the same target
/// names the file uses, so the emitted config and this model cannot spell a
/// target differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Trailer,
    Line,
    Subject,
    Author,
    Committer,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Target::Trailer => "trailer",
            Target::Line => "line",
            Target::Subject => "subject",
            Target::Author => "author",
            Target::Committer => "committer",
        };
        f.write_str(name)
    }
}

/// A validated pattern record with its regex already compiled, so the match
/// engine never recompiles (spec §10).
#[derive(Debug)]
pub struct Pattern {
    pub id: String,
    pub description: String,
    pub target: Target,
    pub key: Option<String>,
    pub regex: Regex,
}

/// The complete pattern list from one config file (spec §3.3: a user file
/// replaces the defaults, it never merges with them).
#[derive(Debug)]
pub struct Config {
    /// Informational in v1: kept so `config show` can print it, never used for
    /// matching. Any TOML value is accepted (spec §3.4).
    pub version: Option<toml::Value>,
    pub patterns: Vec<Pattern>,
}

impl Config {
    /// Parse and validate config text.
    ///
    /// Fails on the first violation in file order, so a user who fixes one
    /// problem sees the next on the following run rather than a wall of errors.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(text)?;

        let capacity = raw.pattern.len();
        let mut patterns = Vec::with_capacity(capacity);
        let mut seen_ids: HashSet<String> = HashSet::with_capacity(capacity);

        for pattern in raw.pattern {
            // spec §3.4: `id` is a stable identifier the tool reports, so a
            // repeat makes a report ambiguous.
            if !seen_ids.insert(pattern.id.clone()) {
                return Err(ConfigError::DuplicateId { id: pattern.id });
            }

            match pattern.target {
                Target::Trailer if pattern.key.is_none() => {
                    return Err(ConfigError::MissingKey { id: pattern.id });
                }
                target if target != Target::Trailer && pattern.key.is_some() => {
                    return Err(ConfigError::UnexpectedKey {
                        id: pattern.id,
                        target,
                    });
                }
                _ => {}
            }

            let regex = Regex::new(&pattern.regex).map_err(|source| ConfigError::InvalidRegex {
                id: pattern.id.clone(),
                source,
            })?;

            patterns.push(Pattern {
                id: pattern.id,
                description: pattern.description,
                target: pattern.target,
                key: pattern.key,
                regex,
            });
        }

        Ok(Config {
            version: raw.version,
            patterns,
        })
    }

    /// The loaded config as TOML, for `config show` (spec §6).
    ///
    /// Rendered from the loaded model, not from the file text: hand-edited
    /// spacing, single quotes and comments do not survive, the records do. Field
    /// order follows the schema example in spec §3.4, so the output reads like
    /// the document a user is expected to write.
    ///
    /// `version` is written only when the file carried one: this reports what is
    /// in effect, and a file without a version keeps none (spec §3.4).
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        let pattern: Vec<RenderedPattern<'_>> = self
            .patterns
            .iter()
            .map(|pattern| RenderedPattern {
                id: &pattern.id,
                description: &pattern.description,
                target: pattern.target,
                // Only trailer patterns carry a key, because this model rejects
                // a key on any other target, so writing one would emit a
                // document that `Config::parse` refuses (spec §3.4).
                key: pattern.key.as_deref(),
                // A compiled regex cannot be written back, and `as_str` is the
                // text it was compiled from.
                regex: pattern.regex.as_str(),
            })
            .collect();

        toml::to_string(&RenderedFile {
            version: self.version.as_ref(),
            pattern,
        })
    }
}

/// The document `config show` prints for a loaded config.
#[derive(Serialize)]
struct RenderedFile<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<&'a toml::Value>,
    /// Serialized as a TOML array of tables, `[[pattern]]` (spec §3.4).
    #[serde(rename = "pattern")]
    pattern: Vec<RenderedPattern<'a>>,
}

#[derive(Serialize)]
struct RenderedPattern<'a> {
    id: &'a str,
    description: &'a str,
    target: Target,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<&'a str>,
    regex: &'a str,
}

/// A config file that cannot be used. Every variant is a config error, which
/// spec §6 maps to exit status 2.
#[derive(Debug)]
pub enum ConfigError {
    /// Not valid TOML, or a missing, unknown or wrongly typed field.
    Syntax(toml::de::Error),
    DuplicateId {
        id: String,
    },
    MissingKey {
        id: String,
    },
    UnexpectedKey {
        id: String,
        target: Target,
    },
    InvalidRegex {
        id: String,
        source: regex::Error,
    },
}

impl ConfigError {
    /// spec §6: a config error exits 2, and the hook and CI branch on that.
    pub fn exit_code(&self) -> u8 {
        2
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Syntax(source) => write!(f, "invalid config: {source}"),
            ConfigError::DuplicateId { id } => {
                write!(f, "pattern \"{id}\": duplicate id")
            }
            ConfigError::MissingKey { id } => write!(
                f,
                "pattern \"{id}\": target \"trailer\" requires a \"key\" field"
            ),
            ConfigError::UnexpectedKey { id, target } => write!(
                f,
                "pattern \"{id}\": target \"{target}\" does not take a \"key\" field"
            ),
            ConfigError::InvalidRegex { id, source } => {
                write!(f, "pattern \"{id}\": invalid \"regex\": {source}")
            }
        }
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(source: toml::de::Error) -> Self {
        ConfigError::Syntax(source)
    }
}

/// The file as written. `deny_unknown_fields` implements the spec §3.4 rule
/// that a typo cannot silently disable a rule.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    version: Option<toml::Value>,
    #[serde(default)]
    pattern: Vec<RawPattern>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPattern {
    id: String,
    description: String,
    target: Target,
    key: Option<String>,
    regex: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
version = 1

[[pattern]]
id = "co-authored-by.claude"
description = "Co-authored-by trailer naming Claude"
target = "trailer"
key = "co-authored-by"
regex = '(?i)(\bclaude\b|@anthropic\.com)'

[[pattern]]
id = "aider.subject"
description = "Aider subject prefix"
target = "subject"
regex = '(?i)^aider\s*:'
"#;

    fn error(text: &str) -> ConfigError {
        match Config::parse(text) {
            Ok(config) => panic!(
                "expected a rejection, loaded {} patterns",
                config.patterns.len()
            ),
            Err(err) => err,
        }
    }

    fn assert_exit_2(err: &ConfigError) {
        assert_eq!(err.exit_code(), 2, "config errors exit 2: {err}");
    }

    #[test]
    fn valid_config_loads() {
        let config = Config::parse(VALID).expect("valid config loads");

        assert_eq!(config.version.and_then(|value| value.as_integer()), Some(1));
        assert_eq!(config.patterns.len(), 2);

        let trailer = &config.patterns[0];
        assert_eq!(trailer.id, "co-authored-by.claude");
        assert_eq!(trailer.description, "Co-authored-by trailer naming Claude");
        assert_eq!(trailer.target, Target::Trailer);
        assert_eq!(trailer.key.as_deref(), Some("co-authored-by"));
        assert!(trailer.regex.is_match("Claude <noreply@anthropic.com>"));

        let subject = &config.patterns[1];
        assert_eq!(subject.target, Target::Subject);
        assert_eq!(subject.key, None);
        assert!(subject.regex.is_match("aider: fix validation"));
    }

    #[test]
    fn every_target_name_deserializes() {
        for (name, expected) in [
            ("trailer", Target::Trailer),
            ("line", Target::Line),
            ("subject", Target::Subject),
            ("author", Target::Author),
            ("committer", Target::Committer),
        ] {
            let key = if expected == Target::Trailer {
                "key = \"assisted-by\"\n"
            } else {
                ""
            };
            let text = format!(
                "[[pattern]]\nid = \"p\"\ndescription = \"d\"\ntarget = \"{name}\"\n{key}regex = \"\\\\S\"\n"
            );
            let config = Config::parse(&text).unwrap_or_else(|err| panic!("{name}: {err}"));
            assert_eq!(config.patterns[0].target, expected, "target {name}");
        }
    }

    #[test]
    fn unknown_field_is_rejected() {
        let err = error(
            r#"
[[pattern]]
id = "p"
description = "d"
target = "line"
regex = '\S'
enabled = true
"#,
        );
        let message = err.to_string();
        assert!(message.contains("unknown field"), "{message}");
        assert!(message.contains("enabled"), "{message}");
        assert_exit_2(&err);
    }

    #[test]
    fn unknown_top_level_field_is_rejected() {
        let err = error("patterns = []\n");
        let message = err.to_string();
        assert!(message.contains("unknown field"), "{message}");
        assert!(message.contains("patterns"), "{message}");
        assert_exit_2(&err);
    }

    #[test]
    fn unknown_target_is_rejected() {
        let err = error(
            r#"
[[pattern]]
id = "p"
description = "d"
target = "footer"
regex = '\S'
"#,
        );
        let message = err.to_string();
        assert!(message.contains("unknown variant"), "{message}");
        assert!(message.contains("footer"), "{message}");
        assert_exit_2(&err);
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let err = error(
            r#"
[[pattern]]
id = "twice"
description = "first"
target = "line"
regex = '\S'

[[pattern]]
id = "twice"
description = "second"
target = "line"
regex = '\S'
"#,
        );
        let message = err.to_string();
        assert!(message.contains("twice"), "{message}");
        assert!(message.contains("duplicate"), "{message}");
        assert_exit_2(&err);
    }

    #[test]
    fn trailer_pattern_without_key_is_rejected() {
        let err = error(
            r#"
[[pattern]]
id = "co-authored-by.claude"
description = "d"
target = "trailer"
regex = '\S'
"#,
        );
        let message = err.to_string();
        assert!(message.contains("co-authored-by.claude"), "{message}");
        assert!(message.contains("key"), "{message}");
        assert!(message.contains("trailer"), "{message}");
        assert_exit_2(&err);
    }

    #[test]
    fn key_on_a_non_trailer_pattern_is_rejected() {
        let err = error(
            r#"
[[pattern]]
id = "free-form-footer.url"
description = "d"
target = "line"
key = "co-authored-by"
regex = '\S'
"#,
        );
        let message = err.to_string();
        assert!(message.contains("free-form-footer.url"), "{message}");
        assert!(message.contains("key"), "{message}");
        assert!(message.contains("line"), "{message}");
        assert_exit_2(&err);
    }

    #[test]
    fn invalid_regex_is_rejected() {
        let err = error(
            r#"
[[pattern]]
id = "broken"
description = "d"
target = "line"
regex = "(unclosed"
"#,
        );
        let message = err.to_string();
        assert!(message.contains("broken"), "{message}");
        assert!(message.contains("regex"), "{message}");
        assert_exit_2(&err);
    }

    #[test]
    fn missing_regex_is_rejected() {
        let err = error(
            r#"
[[pattern]]
id = "no-regex"
description = "d"
target = "line"
"#,
        );
        let message = err.to_string();
        assert!(message.contains("regex"), "{message}");
        assert_exit_2(&err);
    }

    #[test]
    fn unknown_version_value_is_accepted() {
        let config =
            Config::parse("version = \"whatever-v2\"\n").expect("version is informational");
        assert_eq!(
            config.version.as_ref().and_then(toml::Value::as_str),
            Some("whatever-v2")
        );
        assert!(config.patterns.is_empty());
    }

    #[test]
    fn missing_version_is_accepted() {
        let config = Config::parse("").expect("an empty file is valid");
        assert!(config.version.is_none());
        assert!(config.patterns.is_empty());
    }

    #[test]
    fn empty_pattern_list_is_accepted() {
        let config =
            Config::parse("# every pattern commented out\nversion = 1\n").expect("valid config");
        assert!(
            config.patterns.is_empty(),
            "spec §3.3: no patterns is valid"
        );
    }

    #[test]
    fn compiled_regex_list_has_one_entry_per_pattern() {
        let text = r#"
[[pattern]]
id = "one"
description = "d"
target = "line"
regex = "^a"

[[pattern]]
id = "two"
description = "d"
target = "subject"
regex = "^b"

[[pattern]]
id = "three"
description = "d"
target = "trailer"
key = "assisted-by"
regex = '\S'
"#;
        let config = Config::parse(text).expect("valid config");

        assert_eq!(config.patterns.len(), 3);
        for (pattern, source) in config.patterns.iter().zip(["^a", "^b", r"\S"]) {
            assert_eq!(pattern.regex.as_str(), source, "pattern {}", pattern.id);
        }
    }

    #[test]
    fn every_rejection_exits_2() {
        let rejections = [
            "recurse = true\n",
            "[[pattern]]\nid = \"a\"\ndescription = \"d\"\ntarget = \"line\"\nregex = \"a\"\n\n[[pattern]]\nid = \"a\"\ndescription = \"d\"\ntarget = \"line\"\nregex = \"a\"\n",
            "[[pattern]]\nid = \"a\"\ndescription = \"d\"\ntarget = \"trailer\"\nregex = \"a\"\n",
            "[[pattern]]\nid = \"a\"\ndescription = \"d\"\ntarget = \"subject\"\nkey = \"k\"\nregex = \"a\"\n",
            "[[pattern]]\nid = \"a\"\ndescription = \"d\"\ntarget = \"line\"\nregex = \"(\"\n",
        ];

        for text in rejections {
            let err = error(text);
            assert_eq!(err.exit_code(), 2, "{text}: {err}");
            assert!(
                matches!(
                    err,
                    ConfigError::Syntax(_)
                        | ConfigError::DuplicateId { .. }
                        | ConfigError::MissingKey { .. }
                        | ConfigError::UnexpectedKey { .. }
                        | ConfigError::InvalidRegex { .. }
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn to_toml_round_trips_the_loaded_model() {
        // `config show` prints this text, and the printed text must be a config
        // the binary can apply, so every field has to survive the round trip.
        let config = Config::parse(VALID).expect("valid config");
        let text = config.to_toml().expect("the model serializes");
        let reloaded = Config::parse(&text).unwrap_or_else(|err| panic!("{err}\n{text}"));

        assert_eq!(
            reloaded.version.and_then(|value| value.as_integer()),
            Some(1)
        );
        assert_eq!(reloaded.patterns.len(), config.patterns.len());
        for (before, after) in config.patterns.iter().zip(&reloaded.patterns) {
            assert_eq!(after.id, before.id);
            assert_eq!(after.description, before.description, "{}", before.id);
            assert_eq!(after.target, before.target, "{}", before.id);
            assert_eq!(after.key, before.key, "{}", before.id);
            assert_eq!(
                after.regex.as_str(),
                before.regex.as_str(),
                "{}: the regex text is the compiled source",
                before.id
            );
        }
    }

    #[test]
    fn to_toml_writes_the_schema_field_order() {
        let config = Config::parse(VALID).expect("valid config");
        let text = config.to_toml().expect("the model serializes");

        assert!(text.starts_with("version = 1\n"), "{text}");
        let first = text
            .find("[[pattern]]")
            .unwrap_or_else(|| panic!("a pattern table: {text}"));
        let header = &text[first..];
        let id = header.find("id = \"").expect("id is emitted");
        let description = header
            .find("description = ")
            .expect("description is emitted");
        let target = header.find("target = ").expect("target is emitted");
        let key = header.find("key = ").expect("key is emitted");
        let regex = header.find("regex = ").expect("regex is emitted");
        assert!(
            id < description && description < target && target < key && key < regex,
            "spec §3.4 order is id, description, target, key, regex:\n{header}"
        );
    }

    #[test]
    fn to_toml_omits_a_version_the_file_did_not_carry() {
        // spec §3.4: `version` is informational, so a file without one must not
        // grow a `version = 1` in `config show`.
        let config = Config::parse(
            "[[pattern]]\nid = \"p\"\ndescription = \"d\"\ntarget = \"line\"\nregex = \"x\"\n",
        )
        .expect("valid config");
        let text = config.to_toml().expect("the model serializes");

        assert!(!text.contains("version"), "{text}");
        assert!(Config::parse(&text).is_ok(), "{text}");
    }

    #[test]
    fn to_toml_keeps_an_unusual_version_value() {
        // The field is `toml::Value`, so a value this crate cannot name must
        // still be reported rather than dropped or normalized.
        let config = Config::parse("version = [1, 2]\n").expect("version accepts any TOML value");
        let text = config.to_toml().expect("the model serializes");
        let reloaded = Config::parse(&text).unwrap_or_else(|err| panic!("{err}\n{text}"));

        assert_eq!(reloaded.version, config.version, "{text}");
    }

    #[test]
    fn to_toml_of_an_empty_pattern_list_is_a_loadable_document() {
        // spec §3.3: a file with no `[[pattern]]` entry disables all checking,
        // so `config show` prints no table and the result still loads.
        let config = Config::parse("version = 1\n").expect("valid config");
        let text = config.to_toml().expect("the model serializes");

        assert!(!text.contains("[[pattern]]"), "{text}");
        let reloaded = Config::parse(&text).unwrap_or_else(|err| panic!("{err}\n{text}"));
        assert!(reloaded.patterns.is_empty());
    }

    #[test]
    fn first_violation_in_file_order_is_reported() {
        let err = error(
            r#"
[[pattern]]
id = "later"
description = "d"
target = "trailer"
regex = '\S'

[[pattern]]
id = "first"
description = "d"
target = "line"
key = "nope"
regex = '\S'
"#,
        );
        assert!(
            err.to_string().contains("later"),
            "the earlier pattern is reported first: {err}"
        );
    }
}
