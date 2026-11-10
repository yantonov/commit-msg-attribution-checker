//! The `verify` report (spec §5.4): the recorded matches in, the block out.
//!
//! [`render`] answers with `None` for an empty list, so "a clean message writes
//! nothing" is a property of the return type rather than a branch the caller
//! has to remember. The whole block is one string: the caller writes it and
//! decides no formatting, which is also why `-v` cannot change stdout.
//!
//! The wording is the spec's, so each piece of it appears exactly once here: the
//! header, the position of a match, the stanza shape, and the singular of the
//! closing count.

use crate::config::Target;
use crate::engine::Match;

/// The line that opens a report. It carries the tool's full name, because a
/// hook log shows this text (spec §5.4).
const HEADER: &str = "commit-msg-attribution-checker: attribution detected";

/// The closing line: the count, then the verdict.
///
/// spec §5.4 shows `2 matches.` and does not spell the singular; `1 match.` is
/// chosen for grammar. Built here so a spec change to either form is one edit.
fn closing(count: usize) -> String {
    match count {
        1 => "1 match. Commit rejected.".to_owned(),
        count => format!("{count} matches. Commit rejected."),
    }
}

/// The block for one run, or `None` when there is nothing to report.
pub fn render(matches: &[Match]) -> Option<String> {
    if matches.is_empty() {
        return None;
    }

    // One blank line after the header and one after every stanza. The last of
    // those is the blank line spec §5.4 puts before the closing line, so the
    // separator is not special-cased.
    let mut block = format!("{HEADER}\n\n");
    for found in matches {
        block.push_str(&format!("  {} ({})\n", found.id, position(found)));
        block.push_str(&format!("    {}\n", found.description));
        block.push_str(&format!("    value: {}\n", found.value));
        block.push('\n');
    }
    block.push_str(&closing(matches.len()));
    block.push('\n');
    Some(block)
}

/// Where a match applies, in the parentheses of its stanza.
///
/// A message match prints `line N` first, then the target alongside the one word
/// the target adds: the trailer key for a trailer match (spec §5.4 prints
/// `trailer co-authored-by`), and the target's own name otherwise — `subject`
/// and `line` follow that reading, which spec §5.4 does not show but §5.2 names.
///
/// An identity match has no message line (spec §5.3), so it prints the bare
/// target, `author` or `committer`, and no parenthesised position is invented.
fn position(found: &Match) -> String {
    let Some(line) = found.line else {
        return found.target.to_string();
    };

    let detail = match (&found.target, &found.trailer_key) {
        (Target::Trailer, Some(key)) => format!("trailer {key}"),
        // The engine records a trailer match with its key, so this arm only
        // keeps the function total; the target name is still the true word.
        _ => found.target.to_string(),
    };
    format!("line {line}, {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defaults::default_patterns;
    use crate::engine;
    use crate::message::Message;
    use std::path::Path;

    fn found(
        id: &str,
        description: &str,
        target: Target,
        value: &str,
        line: Option<usize>,
        trailer_key: Option<&str>,
    ) -> Match {
        Match {
            id: id.to_owned(),
            description: description.to_owned(),
            target,
            value: value.to_owned(),
            line,
            trailer_key: trailer_key.map(str::to_owned),
        }
    }

    /// The two records of the block in spec §5.4, built by hand so this test
    /// states the expected bytes instead of deriving them from the engine.
    fn spec_example() -> Vec<Match> {
        vec![
            found(
                "co-authored-by.claude",
                "Co-authored-by trailer naming Claude",
                Target::Trailer,
                "Claude <noreply@anthropic.com>",
                Some(4),
                Some("co-authored-by"),
            ),
            found(
                "amp-thread-id",
                "Provenance link to an Amp thread",
                Target::Trailer,
                "https://ampcode.com/threads/T-019",
                Some(5),
                Some("amp-thread-id"),
            ),
        ]
    }

    #[test]
    fn the_block_is_the_one_in_the_spec() {
        // spec §5.4, character for character: header, blank line, a stanza per
        // match, a blank line before the closing line.
        let expected = "\
commit-msg-attribution-checker: attribution detected

  co-authored-by.claude (line 4, trailer co-authored-by)
    Co-authored-by trailer naming Claude
    value: Claude <noreply@anthropic.com>

  amp-thread-id (line 5, trailer amp-thread-id)
    Provenance link to an Amp thread
    value: https://ampcode.com/threads/T-019

2 matches. Commit rejected.
";

        assert_eq!(render(&spec_example()).as_deref(), Some(expected));
    }

    #[test]
    fn the_engine_over_the_spec_message_produces_that_block() {
        // The block above, but reached through matching, so the line numbers are
        // spec §5.3's and the field order is not the test's own.
        let message = Message::parse(
            b"Subject line\n\nBody\n\
              Co-authored-by: Claude <noreply@anthropic.com>\n\
              amp-thread-id: https://ampcode.com/threads/T-019\n",
            Path::new("COMMIT_EDITMSG"),
        )
        .expect("the fixture parses");

        let matches = engine::run(&default_patterns(), &message, None);

        assert_eq!(render(&matches), render(&spec_example()));
    }

    #[test]
    fn a_clean_message_renders_nothing_at_all() {
        // spec §5.4: not even a newline. `None` is what keeps the caller from
        // writing one.
        assert_eq!(render(&[]), None);
    }

    #[test]
    fn one_match_closes_with_the_singular() {
        let block = render(&spec_example()[..1]).expect("one match reports");

        assert!(
            block.ends_with("\n\n1 match. Commit rejected.\n"),
            "{block:?}"
        );
    }

    #[test]
    fn a_subject_match_is_positioned_by_line_and_target() {
        // spec §5.2 names the target `subject`; §5.4 shows only the trailer
        // form, so the same shape applies with the target's own word.
        let block = render(&[found(
            "aider.subject",
            "Aider subject prefix",
            Target::Subject,
            "aider: fix validation",
            Some(1),
            None,
        )])
        .expect("one match reports");

        assert!(
            block.contains("  aider.subject (line 1, subject)\n"),
            "{block}"
        );
    }

    #[test]
    fn a_line_match_is_positioned_by_line_and_target() {
        let block = render(&[found(
            "free-form-footer.tool",
            "Attribution footer naming a known tool or model",
            Target::Line,
            "Made with Cursor",
            Some(3),
            None,
        )])
        .expect("one match reports");

        assert!(
            block.contains("  free-form-footer.tool (line 3, line)\n"),
            "{block}"
        );
    }

    #[test]
    fn an_identity_match_prints_the_bare_target() {
        // spec §5.3: an identity is commit metadata, so there is no line to
        // report and no placeholder number is invented.
        for (target, name) in [(Target::Author, "author"), (Target::Committer, "committer")] {
            let block = render(&[found(
                "aider.author-name",
                "Aider author-name decoration",
                target,
                "Aider (aider)",
                None,
                None,
            )])
            .expect("one match reports");

            assert!(
                block.contains(&format!("  aider.author-name ({name})\n")),
                "{block}"
            );
            assert!(!block.contains("line"), "{block}");
        }
    }

    #[test]
    fn the_count_matches_the_number_of_stanzas() {
        let mut matches = spec_example();
        matches.push(found(
            "made-with",
            "Made-with trailer naming a tool",
            Target::Trailer,
            "Cursor",
            Some(6),
            Some("made-with"),
        ));

        let block = render(&matches).expect("three matches report");

        // A stanza's first line is indented two spaces; its description and
        // value lines are indented four, so the shallower indent counts stanzas.
        let stanzas = block
            .lines()
            .filter(|line| line.starts_with("  ") && !line.starts_with("    "))
            .count();
        assert_eq!(stanzas, 3, "{block}");
        assert!(block.ends_with("3 matches. Commit rejected.\n"), "{block}");
    }
}
