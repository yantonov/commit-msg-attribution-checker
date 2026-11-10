//! The match engine (spec §5.2, §5.3): a loaded config and a parsed message
//! in, the ordered match list out.
//!
//! One rule shapes this module: the trailer gate. A trailer pattern runs first
//! on every line, and a line it matches is closed to the `line` and `subject`
//! patterns, so one attribution on one line is reported once (spec §4.4,
//! §8 criterion 6). The gate is a trailer *match*, not trailer *shape*, which
//! is why `aider: fix validation` still reaches `aider.subject` (spec §5.2).
//!
//! Regexes arrive compiled from [`Config`], and nothing here shells out: v1
//! implements the trailer parser itself, so matching needs no repository and no
//! git binary (spec §5.2).

// Reachable once `verify` lands; clippy runs with -D warnings, so the module
// stays quiet until then.
#![allow(dead_code)]

use crate::config::{Config, Pattern, Target};
use crate::message::{Line, Message};

/// One recorded match (spec §5.3).
///
/// The fields are exactly what the spec §5.4 block prints, so the renderer never
/// reads back into the config or the message: `id` and `description` name and
/// explain the pattern, `target` and `trailer_key` build the
/// `(line N, trailer KEY)` position, `value` is the text the regex was tested
/// against, and `line` is the message line number.
///
/// `trailer_key` is the key as the *config* spells it, because spec §5.4 prints
/// `trailer co-authored-by` for a message line written `Co-authored-by:`. The
/// target is `trailer` either way; the key is the extra word the block shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub id: String,
    pub description: String,
    pub target: Target,
    /// The trailer value for a `trailer` pattern, the whole line for `line` and
    /// `subject` (spec §5.2).
    pub value: String,
    /// 1-based and counted over the whole file, comment lines included
    /// (spec §5.3).
    pub line: usize,
    /// `None` for a non-trailer pattern. Identity matches would carry no message
    /// line at all, so the `identity-targets` ticket extends this record rather
    /// than reusing `line: 0`.
    pub trailer_key: Option<String>,
}

impl Match {
    fn new(pattern: &Pattern, value: String, line: usize, trailer_key: Option<String>) -> Self {
        Self {
            id: pattern.id.clone(),
            description: pattern.description.clone(),
            target: pattern.target,
            value,
            line,
            trailer_key,
        }
    }
}

/// Run every pattern against the message (spec §5.2).
///
/// Records at most one match per pattern: the earliest line where it matches.
/// Three tools credited on three lines are three different patterns, so they
/// produce three records; one pattern that matches on two lines produces one
/// (spec §5.3).
///
/// The order is the order the engine evaluates, and it is stable, so the output
/// block and the JSON are deterministic:
///
/// 1. line by line, ascending line number, over the non-comment lines;
/// 2. within one line, config order — the order the patterns appear in the
///    file.
///
/// Rule 2 needs no tie-break between the targets, because the trailer gate
/// means a line yields either trailer records or `line` and `subject` records,
/// never both: a trailer match skips the whole rest of the line.
///
/// Identity is not resolved here. `author` and `committer` patterns take their
/// value from `git var`, which this module does not call (spec §5.2), so they
/// contribute no records; the `identity-targets` ticket adds that input.
pub fn run(config: &Config, message: &Message) -> Vec<Match> {
    let subject = message.subject().map(|line| line.number);
    // One slot per pattern, since spec §5.3 records a pattern at most once.
    // Indexed by position rather than keyed by `id`, so the engine never hashes
    // for it.
    let mut recorded = vec![false; config.patterns.len()];
    let mut matches = Vec::new();

    for line in message.lines_to_match() {
        let claimed = trailer_pass(config, line, &mut recorded, &mut matches);

        // spec §5.2: a claimed line is closed to the `line` and the `subject`
        // patterns alike, so the skip of one is the skip of the other.
        if claimed {
            continue;
        }

        let is_subject = subject == Some(line.number);
        for (index, pattern) in config.patterns.iter().enumerate() {
            let targeted = match pattern.target {
                Target::Line => true,
                Target::Subject => is_subject,
                // `trailer` already ran in its own pass; identity targets read
                // `git var` instead of message text (spec §5.2).
                Target::Trailer | Target::Author | Target::Committer => false,
            };
            if !targeted || recorded[index] || !pattern.regex.is_match(&line.text) {
                continue;
            }
            recorded[index] = true;
            matches.push(Match::new(pattern, line.text.clone(), line.number, None));
        }
    }

    matches
}

/// The trailer pass of spec §5.2 for one line: run every trailer pattern and
/// report whether any of them claimed the line.
///
/// A claim is recorded even when the matching pattern already has its record.
/// The gate decides which targets may look at the line at all; a second
/// `Generated-by:` line is still claimed, even though it adds no match.
fn trailer_pass(
    config: &Config,
    line: &Line,
    recorded: &mut [bool],
    matches: &mut Vec<Match>,
) -> bool {
    // No trailer shape means nothing for this pass to test, and no claim. A
    // comment line offers no trailer either, because `#` cannot start a key
    // (spec §5.1).
    let Some(trailer) = &line.trailer else {
        return false;
    };

    let mut claimed = false;
    for (index, pattern) in config.patterns.iter().enumerate() {
        if pattern.target != Target::Trailer {
            continue;
        }
        // The config model rejects a trailer pattern without a key (spec §3.4),
        // so this fallback only keeps the engine total.
        let Some(key) = pattern.key.as_deref() else {
            continue;
        };
        if !trailer.key_is(key) || !pattern.regex.is_match(&trailer.value) {
            continue;
        }

        claimed = true;
        if recorded[index] {
            continue;
        }
        recorded[index] = true;
        matches.push(Match::new(
            pattern,
            trailer.value.clone(),
            line.number,
            Some(key.to_owned()),
        ));
    }

    claimed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defaults::default_patterns;
    use std::path::Path;

    fn message(text: &str) -> Message {
        Message::parse(text.as_bytes(), Path::new("COMMIT_EDITMSG"))
            .unwrap_or_else(|err| panic!("the test message parses: {err}"))
    }

    fn config(text: &str) -> Config {
        Config::parse(text).unwrap_or_else(|err| panic!("the test config loads: {err}"))
    }

    /// Run against the shipped registry, so a test asserts the patterns users
    /// actually get (spec §4).
    fn run_defaults(text: &str) -> Vec<Match> {
        run(&default_patterns(), &message(text))
    }

    fn ids(matches: &[Match]) -> Vec<&str> {
        matches.iter().map(|found| found.id.as_str()).collect()
    }

    fn numbers(matches: &[Match]) -> Vec<usize> {
        matches.iter().map(|found| found.line).collect()
    }

    fn pattern<'a>(config: &'a Config, id: &str) -> &'a Pattern {
        config
            .patterns
            .iter()
            .find(|pattern| pattern.id == id)
            .unwrap_or_else(|| panic!("the config holds {id}"))
    }

    #[test]
    fn criterion_1_a_claude_co_author_trailer_matches() {
        let found = run_defaults("Subject\n\nCo-authored-by: Claude <noreply@anthropic.com>\n");

        assert_eq!(ids(&found), ["co-authored-by.claude"], "{found:#?}");
        let matched = &found[0];
        assert_eq!(matched.description, "Co-authored-by trailer naming Claude");
        assert_eq!(matched.target, Target::Trailer);
        assert_eq!(matched.value, "Claude <noreply@anthropic.com>");
        assert_eq!(matched.line, 3);
        assert_eq!(
            matched.trailer_key.as_deref(),
            Some("co-authored-by"),
            "spec §5.4 prints the trailer key next to the target"
        );
    }

    #[test]
    fn criterion_2_a_human_co_author_does_not_match() {
        let found = run_defaults("Subject\n\nCo-authored-by: Alice <alice@example.com>\n");

        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn criterion_3_three_attribution_lines_yield_three_matches() {
        let found = run_defaults(
            "Add the parser\n\n\
             Co-authored-by: Claude <noreply@anthropic.com>\n\
             Made-with: Cursor\n\
             🤖 Generated with Amp\n",
        );

        assert_eq!(
            ids(&found),
            [
                "co-authored-by.claude",
                "made-with",
                "free-form-footer.emoji"
            ],
            "{found:#?}"
        );
        assert_eq!(numbers(&found), [3, 4, 5]);
    }

    #[test]
    fn criterion_5_prose_about_a_build_is_clean() {
        // spec §4.4: the attribution phrase alone is not attribution, so none of
        // these may report.
        for line in [
            "generated by the build script",
            "Created by hand",
            "written by the release team",
            "Made by Nix",
            "generated with protoc",
            "implemented using a visitor pattern",
        ] {
            let found = run_defaults(&format!("Subject\n\n{line}\n"));
            assert!(found.is_empty(), "{line}: {found:#?}");
        }
    }

    #[test]
    fn criterion_5_made_with_cursor_matches_the_footer_target() {
        let found = run_defaults("Subject\n\nMade with Cursor\n");

        assert_eq!(ids(&found), ["free-form-footer.tool"], "{found:#?}");
        let matched = &found[0];
        assert_eq!(matched.target, Target::Line);
        assert_eq!(
            matched.value, "Made with Cursor",
            "a line pattern's value is the whole line"
        );
        assert_eq!(matched.line, 3);
        assert_eq!(matched.trailer_key, None);
    }

    #[test]
    fn criterion_5_the_robot_marker_matches_even_without_a_known_tool() {
        let found = run_defaults("Subject\n\n🤖 Generated with something unknown\n");

        assert_eq!(ids(&found), ["free-form-footer.emoji"], "{found:#?}");
        assert_eq!(found[0].line, 3);
    }

    #[test]
    fn criterion_6_assisted_by_amp_reports_one_match() {
        let found = run_defaults("Subject\n\nAssisted-by: Amp <amp@ampcode.com>\n");

        assert_eq!(ids(&found), ["assisted-by"], "{found:#?}");
        assert_eq!(found[0].value, "Amp <amp@ampcode.com>");
    }

    #[test]
    fn the_trailer_gate_keeps_the_footer_pattern_off_a_claimed_line() {
        // `Generated-by: Cursor` is trailer shape, names a tool, and matches the
        // `generated-by` trailer pattern. Without the gate the footer pattern
        // would report the same attribution a second time (spec §4.4, §5.2).
        let config = default_patterns();
        assert!(
            pattern(&config, "free-form-footer.tool")
                .regex
                .is_match("Generated-by: Cursor"),
            "the footer pattern would match this line, so the gate is what stops it"
        );

        let found = run(&config, &message("Subject\n\nGenerated-by: Cursor\n"));

        assert_eq!(ids(&found), ["generated-by"], "{found:#?}");
        assert_eq!(found[0].line, 3);
    }

    #[test]
    fn a_gated_line_is_skipped_by_the_line_patterns_too() {
        // `Made-with: Cursor` reads as a footer as well, so the same line would
        // report twice without the gate (spec §5.3 states the case).
        let config = default_patterns();
        assert!(
            pattern(&config, "free-form-footer.tool")
                .regex
                .is_match("Made-with: Cursor"),
            "the footer pattern would match this line"
        );

        let found = run(&config, &message("Subject\n\nMade-with: Cursor\n"));

        assert_eq!(ids(&found), ["made-with"], "{found:#?}");
    }

    #[test]
    fn criterion_7_aider_subject_is_reported_despite_trailer_shape() {
        let found = run_defaults("aider: fix validation\n");

        assert_eq!(ids(&found), ["aider.subject"], "{found:#?}");
        let matched = &found[0];
        assert_eq!(matched.target, Target::Subject);
        assert_eq!(matched.line, 1);
        assert_eq!(matched.value, "aider: fix validation");
        assert_eq!(matched.trailer_key, None);
    }

    #[test]
    fn trailer_shape_alone_does_not_gate_a_line() {
        // The config has a trailer pattern, so the line has a key the engine
        // could compare — but not this key. Only a trailer match gates.
        let config = config(
            "[[pattern]]\nid = \"co-authored-by.claude\"\ndescription = \"d\"\n\
             target = \"trailer\"\nkey = \"co-authored-by\"\nregex = '(?i)claude'\n\n\
             [[pattern]]\nid = \"aider.subject\"\ndescription = \"d\"\n\
             target = \"subject\"\nregex = '(?i)^aider\\s*:'\n",
        );
        let found = run(&config, &message("aider: fix validation\n"));

        assert_eq!(ids(&found), ["aider.subject"], "{found:#?}");
    }

    #[test]
    fn a_claimed_subject_line_is_skipped_by_the_subject_patterns() {
        // spec §5.2: the subject skip is the same gate as the line skip, so a
        // trailer pattern that claims the subject line closes it to both.
        let config = config(
            "[[pattern]]\nid = \"assisted-by\"\ndescription = \"d\"\n\
             target = \"trailer\"\nkey = \"assisted-by\"\nregex = '\\S'\n\n\
             [[pattern]]\nid = \"subject.any\"\ndescription = \"d\"\n\
             target = \"subject\"\nregex = '\\S'\n",
        );
        let found = run(&config, &message("Assisted-by: Amp <amp@ampcode.com>\n"));

        assert_eq!(ids(&found), ["assisted-by"], "{found:#?}");
    }

    #[test]
    fn the_first_match_per_pattern_is_the_earliest_line() {
        let found = run_defaults(
            "Subject\n\n\
             Co-authored-by: Claude <noreply@anthropic.com>\n\
             \n\
             Co-authored-by: Claude <noreply@anthropic.com>\n",
        );

        assert_eq!(ids(&found), ["co-authored-by.claude"], "{found:#?}");
        assert_eq!(found[0].line, 3, "spec §5.3: the first match is recorded");
    }

    #[test]
    fn a_line_pattern_records_only_its_first_line() {
        let found = run_defaults("Subject\n\nGenerated with Cursor\n\nGenerated with Cursor\n");

        assert_eq!(ids(&found), ["free-form-footer.tool"], "{found:#?}");
        assert_eq!(found[0].line, 3);
    }

    #[test]
    fn a_repeat_of_a_claimed_line_still_gates_when_it_adds_no_record() {
        let found = run_defaults("Subject\n\nGenerated-by: Cursor\n\nGenerated-by: Cursor\n");

        assert_eq!(ids(&found), ["generated-by"], "{found:#?}");
        assert_eq!(
            found[0].line, 3,
            "only the first line is recorded, and the second is still gated"
        );
    }

    #[test]
    fn a_trailer_key_comparison_ignores_case_but_the_record_keeps_the_config_spelling() {
        let found = run_defaults("Subject\n\nCO-AUTHORED-BY: Claude <noreply@anthropic.com>\n");

        assert_eq!(ids(&found), ["co-authored-by.claude"], "{found:#?}");
        assert_eq!(
            found[0].trailer_key.as_deref(),
            Some("co-authored-by"),
            "spec §5.4 prints `trailer co-authored-by`"
        );
        assert_eq!(found[0].value, "Claude <noreply@anthropic.com>");
    }

    #[test]
    fn a_subject_pattern_runs_only_on_the_subject() {
        // The line has trailer shape too, so this also states that neither
        // target claims it: no configured key is `aider`.
        let found = run_defaults("Subject\n\naider: fix validation\n");

        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn comment_lines_and_an_empty_message_yield_no_match() {
        assert!(run_defaults("").is_empty(), "spec §8 criterion 8");
        assert!(run_defaults("# nothing to see\n#\n").is_empty());
        assert!(
            run_defaults("# Co-authored-by: Claude <noreply@anthropic.com>\n").is_empty(),
            "spec §5.1: a `#` in the first column drops the line before matching"
        );
    }

    #[test]
    fn matches_follow_file_order() {
        let found = run_defaults(
            "aider: fix validation\n\n\
             Co-authored-by: Claude <noreply@anthropic.com>\n\
             Generated with Cursor\n",
        );

        assert_eq!(
            ids(&found),
            [
                "aider.subject",
                "co-authored-by.claude",
                "free-form-footer.tool"
            ],
            "{found:#?}"
        );
        assert_eq!(numbers(&found), [1, 3, 4]);
    }

    #[test]
    fn within_one_line_the_config_order_decides() {
        // The engine makes one pass over the config per line, so the file order
        // is the report order and no target gets a ranking of its own.
        let config = config(
            "[[pattern]]\nid = \"subject.aider\"\ndescription = \"d\"\n\
             target = \"subject\"\nregex = '(?i)^aider'\n\n\
             [[pattern]]\nid = \"line.aider\"\ndescription = \"d\"\n\
             target = \"line\"\nregex = '(?i)^aider'\n\n\
             [[pattern]]\nid = \"line.second\"\ndescription = \"d\"\n\
             target = \"line\"\nregex = '(?i)^aider'\n",
        );
        let found = run(&config, &message("aider: fix validation\n"));

        assert_eq!(
            ids(&found),
            ["subject.aider", "line.aider", "line.second"],
            "config order, not target order: {found:#?}"
        );
    }

    #[test]
    fn identity_targets_are_deferred_and_never_read_message_text() {
        // Both lines would match an identity pattern if the engine tested those
        // against message text. Their value is `git var` output (spec §5.2), so
        // the engine reports nothing and `identity-targets` adds the records.
        for text in [
            "Fix the parser (aider)\n",
            "Fix the parser <dev@cursor.com>\n",
        ] {
            let found = run_defaults(text);
            assert!(found.is_empty(), "{text:?}: {found:#?}");
        }
    }

    #[test]
    fn two_runs_over_one_message_produce_the_same_order() {
        let config = default_patterns();
        let parsed = message(
            "aider: fix validation\n\n\
             Co-authored-by: Claude <noreply@anthropic.com>\n\
             Made-with: Cursor\n\
             Generated with Cursor\n\
             🤖 Generated with Amp\n",
        );

        let first = run(&config, &parsed);
        let second = run(&config, &parsed);

        assert_eq!(first, second, "the order must not depend on the run");
        assert_eq!(
            ids(&first),
            [
                "aider.subject",
                "co-authored-by.claude",
                "made-with",
                "free-form-footer.tool",
                "free-form-footer.emoji",
            ],
            "{first:#?}"
        );
        assert_eq!(numbers(&first), [1, 3, 4, 5, 6]);
    }

    #[test]
    fn a_config_without_patterns_matches_nothing() {
        let found = run(
            &config("version = 1\n"),
            &message("Co-authored-by: Claude <noreply@anthropic.com>\n"),
        );

        assert!(found.is_empty());
    }
}
