//! Message parsing (spec §5.1, §5.2): a message file in, the values a pattern
//! tests out.
//!
//! No matching happens here. The parse owns the two rules every target depends
//! on — which lines are comments (§5.1) and which lines carry a trailer
//! (§5.2) — so the match engine tests values and never re-derives shape.
//!
//! Identity targets are not here either: their value comes from `git var`, not
//! from the message file (§5.2).

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

/// The trailer expression fixed by spec §5.2. Compiled once, because every
/// line of every message is tested against it and the expression cannot vary.
static TRAILER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Za-z0-9][A-Za-z0-9-]*):[ \t]*(.+)$")
        .expect("the spec §5.2 trailer expression compiles")
});

/// A trailer-shaped line: the parts of `^([A-Za-z0-9][A-Za-z0-9-]*):[ \t]*(.+)$`
/// (spec §5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trailer {
    /// Stored as written. The comparison ignores case, so lowercasing the
    /// stored text would throw away what the message says.
    pub key: String,
    /// Stored as written after the separator run: the value's case, its inner
    /// spacing and its trailing spaces all survive, because a pattern may test
    /// any of them.
    pub value: String,
}

impl Trailer {
    /// spec §5.2: the key comparison is case-insensitive.
    ///
    /// A comparison instead of a lowercased copy, so a config key and the
    /// message key are never rewritten on the way to a match.
    pub fn key_is(&self, key: &str) -> bool {
        self.key.eq_ignore_ascii_case(key)
    }
}

/// One line of the file with its place in the file.
///
/// A comment line is kept in [`Message::lines`] rather than dropped, because
/// spec §5.3 numbers a match by file line: dropping the line would leave a gap
/// the report has to explain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// 1-based and counts every line of the file, comment lines included
    /// (spec §5.3).
    pub number: usize,
    /// The line without its terminating `\n` and without one trailing `\r`.
    pub text: String,
    /// Whether spec §5.1 drops this line before matching.
    pub comment: bool,
    /// `None` when the line has no trailer shape. Always `None` on a comment
    /// line, because `#` cannot start a key.
    pub trailer: Option<Trailer>,
}

/// A parsed message file.
///
/// The "claimed by a trailer pattern" state of spec §5.2 lives in the match
/// engine, not here: trailer, line and subject patterns all run on one line in
/// one pass, so the gate is a local flag in that pass and outlives nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Every line of the file, comment lines included, in file order.
    pub lines: Vec<Line>,
}

impl Message {
    /// Parse the bytes of a message file.
    ///
    /// `path` is carried only so the UTF-8 error names the file the user has
    /// to fix.
    pub fn parse(bytes: &[u8], path: &Path) -> Result<Self, MessageError> {
        // Decoded whole rather than line by line: a file that is not UTF-8 is
        // unusable in full, so "line 7" would name a position in text the tool
        // cannot read. The file name is the actionable part.
        let text = String::from_utf8(bytes.to_vec()).map_err(|source| MessageError::NotUtf8 {
            path: path.to_path_buf(),
            source,
        })?;

        Ok(Self {
            lines: parse_lines(&text),
        })
    }

    /// Read and parse a message file, so the caller maps one error type for
    /// both the read and the decode (spec §6: exit 2 either way).
    pub fn read(path: &Path) -> Result<Self, MessageError> {
        let bytes = fs::read(path).map_err(|source| MessageError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&bytes, path)
    }

    /// The lines a `line` or `subject` pattern may test (spec §5.1).
    pub fn lines_to_match(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter().filter(|line| !line.comment)
    }

    /// The value of the `subject` target: the first non-comment line
    /// (spec §5.2).
    ///
    /// Literally the first non-comment line, not the first non-blank one, as
    /// the spec words it. An empty file, or one with only comments, has no
    /// subject.
    pub fn subject(&self) -> Option<&Line> {
        self.lines_to_match().next()
    }
}

/// Split decoded text into lines.
fn parse_lines(text: &str) -> Vec<Line> {
    // An empty file has no lines. Splitting first would report one empty line,
    // and an empty line is a value a `line` pattern may match (spec §8
    // criterion 8: an empty file must exit 0).
    if text.is_empty() {
        return Vec::new();
    }

    // A trailing newline terminates the last line; it does not begin a new
    // one. Without this, `a\n` would end in an empty line.
    let body = text.strip_suffix('\n').unwrap_or(text);

    body.split('\n')
        .enumerate()
        .map(|(index, raw)| {
            let comment = is_comment(raw);
            // A CRLF file would otherwise leave `\r` at the end of every value
            // a pattern tests. Stripped here, once, rather than in each
            // pattern.
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            Line {
                number: index + 1,
                text: line.to_owned(),
                comment,
                // No comment guard: `#` cannot start a key, so the expression
                // already declines a comment line (spec §5.1).
                trailer: parse_trailer(line),
            }
        })
        .collect()
}

/// spec §5.1: only a `#` in the first column is a comment. This is git's own
/// rule, so `cleanup=strip` and the check here agree on which lines disappear.
fn is_comment(raw: &str) -> bool {
    // The first byte of the raw line, before any `\r` reasoning: a strip that
    // ran first could not change this answer, and reading it first makes the
    // spec's "first byte" rule visible.
    raw.as_bytes().first() == Some(&b'#')
}

fn parse_trailer(line: &str) -> Option<Trailer> {
    let captures = TRAILER.captures(line)?;
    Some(Trailer {
        key: captures[1].to_owned(),
        // `[ \t]*` is greedy, so it drops the separator run only; trailing
        // spaces stay in the value, exactly as the spec's expression reads.
        value: captures[2].to_owned(),
    })
}

/// A message file that cannot be parsed. Every variant is an I/O error, which
/// spec §6 maps to exit status 2.
#[derive(Debug)]
pub enum MessageError {
    /// The file could not be read: absent, a directory, or a permission error.
    Read { path: PathBuf, source: io::Error },
    /// The file was read but is not UTF-8, so no line can be tested.
    NotUtf8 {
        path: PathBuf,
        source: std::string::FromUtf8Error,
    },
}

impl MessageError {
    /// spec §6: an I/O error exits 2, and the hook and CI branch on that.
    pub fn exit_code(&self) -> u8 {
        2
    }
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageError::Read { path, source } => write!(f, "{}: {source}", path.display()),
            MessageError::NotUtf8 { path, source } => {
                write!(
                    f,
                    "{}: message is not valid UTF-8: {source}",
                    path.display()
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Parse bytes under a stand-in path, so a failure message is checkable.
    fn parsed(bytes: &[u8]) -> Message {
        Message::parse(bytes, Path::new("COMMIT_EDITMSG"))
            .unwrap_or_else(|err| panic!("expected a parse, got {err}"))
    }

    fn parsed_text(text: &str) -> Message {
        parsed(text.as_bytes())
    }

    /// The `(number, text)` pairs, so a test states line numbering directly.
    fn numbered(message: &Message) -> Vec<(usize, &str)> {
        message
            .lines
            .iter()
            .map(|line| (line.number, line.text.as_str()))
            .collect()
    }

    #[test]
    fn hash_in_the_first_column_is_a_comment() {
        let message = parsed_text("# Co-authored-by: Claude <noreply@anthropic.com>\nBody\n");

        assert!(message.lines[0].comment, "spec §5.1");
        assert!(!message.lines[1].comment);
    }

    #[test]
    fn indented_hash_is_kept() {
        // spec §5.1: git's cleanup strips only a `#` in the first column, so
        // this line is part of the message and must be matched.
        let message = parsed_text("  # Co-authored-by: Claude <noreply@anthropic.com>\n");

        assert!(!message.lines[0].comment);
        assert_eq!(
            message.lines[0].text,
            "  # Co-authored-by: Claude <noreply@anthropic.com>"
        );
        assert_eq!(
            message.lines_to_match().count(),
            1,
            "the line reaches the line and subject targets"
        );
    }

    #[test]
    fn hash_inside_a_line_is_kept() {
        let message = parsed_text("body#notcomment\n");

        assert!(!message.lines[0].comment);
        assert_eq!(message.lines[0].text, "body#notcomment");
    }

    #[test]
    fn only_the_first_byte_makes_a_comment() {
        for kept in ["body#notcomment", "  # indented", "\t# tab", " # one space"] {
            let message = parsed_text(kept);
            assert!(!message.lines[0].comment, "{kept:?} is not a comment line");
        }

        for dropped in ["#", "# no space", "#indented"] {
            let message = parsed_text(dropped);
            assert!(message.lines[0].comment, "{dropped:?} is a comment line");
        }
    }

    #[test]
    fn trailer_parse_succeeds_on_the_spec_expression() {
        let message = parsed_text("Co-authored-by: Claude <noreply@anthropic.com>\n");
        let trailer = message.lines[0]
            .trailer
            .as_ref()
            .expect("the line has trailer shape");

        assert_eq!(trailer.key, "Co-authored-by");
        assert_eq!(trailer.value, "Claude <noreply@anthropic.com>");
    }

    #[test]
    fn trailer_key_may_carry_digits_and_hyphens() {
        let message = parsed_text("amp-thread-id: https://ampcode.com/threads/T-019\n");

        assert_eq!(
            message.lines[0].trailer.as_ref().map(|t| t.key.as_str()),
            Some("amp-thread-id")
        );
    }

    #[test]
    fn trailer_parse_fails_without_a_key_and_separator() {
        // Each line is a value a `line` pattern can still test, so a `None`
        // here must not mean the line was dropped.
        for line in [
            "aider fix validation",
            // A blank line is a line; an empty file has none, so that case is
            // tested separately.
            "\n",
            "Key:",
            "co_authored: x",
            "-key: x",
            " key: x",
        ] {
            let message = parsed_text(line);
            assert!(
                message.lines[0].trailer.is_none(),
                "{line:?} does not match the spec §5.2 expression"
            );
            assert!(!message.lines[0].comment, "{line:?} is still matchable");
        }
    }

    #[test]
    fn key_comparison_ignores_case() {
        let message = parsed_text("CO-AUTHORED-BY: Claude <noreply@anthropic.com>\n");
        let trailer = message.lines[0].trailer.as_ref().expect("trailer shape");

        assert_eq!(
            trailer.key, "CO-AUTHORED-BY",
            "the key is stored as written"
        );
        assert!(trailer.key_is("co-authored-by"));
        assert!(trailer.key_is("Co-Authored-By"));
        assert!(!trailer.key_is("assisted-by"));
    }

    #[test]
    fn value_keeps_case_and_spacing_after_the_separator() {
        let message = parsed_text("Made-With:\tMiXeD  Value  \n");
        let trailer = message.lines[0].trailer.as_ref().expect("trailer shape");

        assert_eq!(
            trailer.value, "MiXeD  Value  ",
            "the separator run is dropped; the value's own case and spaces are not"
        );
    }

    #[test]
    fn an_empty_trailer_value_still_carries_one_character() {
        // The spec expression needs one character after `[ \t]*`, so the run
        // backtracks: a whitespace-only value survives as one space. Worth
        // pinning, because `\S` patterns then decline it, as they should.
        let message = parsed_text("Assisted-by:   \n");
        let trailer = message.lines[0].trailer.as_ref().expect("trailer shape");

        assert_eq!(trailer.value, " ");
        assert!(
            TRAILER.captures("Assisted-by:").is_none(),
            "a value with no character at all is not trailer shape"
        );
    }

    #[test]
    fn a_comment_line_offers_no_trailer() {
        // `#` cannot start a key, so the expression declines a comment line on
        // its own and spec §5.1's drop needs no second guard.
        let message = parsed_text("# Co-authored-by: Claude <noreply@anthropic.com>\n");

        assert!(message.lines[0].comment);
        assert!(message.lines[0].trailer.is_none());
    }

    #[test]
    fn line_numbers_start_at_1_and_cover_every_line() {
        let message = parsed_text("# a comment\n\nSubject line\n\nCo-authored-by: Claude\n");

        assert_eq!(
            numbered(&message),
            [
                (1, "# a comment"),
                (2, ""),
                (3, "Subject line"),
                (4, ""),
                (5, "Co-authored-by: Claude"),
            ]
        );
        assert!(message.lines[0].comment);
        assert_eq!(message.lines[4].number, 5, "spec §5.3 counts comments");
    }

    #[test]
    fn an_empty_file_has_no_lines() {
        let message = parsed_text("");

        assert!(message.lines.is_empty(), "spec §8 criterion 8");
        assert!(message.subject().is_none());
        assert_eq!(message.lines_to_match().count(), 0);
    }

    #[test]
    fn a_file_of_comments_only_has_nothing_to_match() {
        let message = parsed_text("# nothing to see\n#\n# Co-authored-by: Claude\n");

        assert_eq!(
            message.lines.len(),
            3,
            "the lines and their numbers survive"
        );
        assert!(message.lines.iter().all(|line| line.comment));
        assert!(message.subject().is_none(), "spec §8 criterion 8");
        assert_eq!(message.lines_to_match().count(), 0);
    }

    #[test]
    fn a_single_newline_is_one_empty_line() {
        // A blank line is not a comment, so it stays a value a `line` pattern
        // may test; only a file with no bytes at all has no lines.
        let message = parsed_text("\n");

        assert_eq!(numbered(&message), [(1, "")]);
        assert_eq!(message.lines_to_match().count(), 1);
    }

    #[test]
    fn a_trailing_newline_does_not_add_a_line() {
        let message = parsed_text("a\n");

        assert_eq!(numbered(&message), [(1, "a")]);
    }

    #[test]
    fn a_last_line_without_a_newline_is_kept() {
        let message = parsed_text("a");

        assert_eq!(numbered(&message), [(1, "a")]);
    }

    #[test]
    fn consecutive_newlines_yield_empty_lines() {
        let message = parsed_text("Subject\n\nBody\n\n");

        assert_eq!(
            numbered(&message),
            [(1, "Subject"), (2, ""), (3, "Body"), (4, "")]
        );
    }

    #[test]
    fn the_subject_is_the_first_non_comment_line() {
        let message = parsed_text("# comment\n\nSubject line\n\nBody\n");

        let subject = message.subject().expect("the message has a subject");
        assert_eq!(subject.number, 2, "the blank line follows the comment");
        assert_eq!(subject.text, "");
    }

    #[test]
    fn crlf_leaves_no_carriage_return_in_a_value() {
        let message =
            parsed_text("Subject\r\n\r\nCo-authored-by: Claude <noreply@anthropic.com>\r\n");

        assert!(
            message.lines.iter().all(|line| !line.text.contains('\r')),
            "{:?}",
            numbered(&message)
        );
        let trailer = message.lines[2]
            .trailer
            .as_ref()
            .expect("the CRLF line has trailer shape");
        assert_eq!(trailer.value, "Claude <noreply@anthropic.com>");
    }

    #[test]
    fn crlf_without_a_final_newline_leaves_no_carriage_return() {
        let message = parsed_text("Subject\r\nCo-authored-by: Claude\r");

        assert_eq!(message.lines[0].text, "Subject");
        assert_eq!(
            message.lines[1].trailer.as_ref().map(|t| t.value.as_str()),
            Some("Claude")
        );
    }

    #[test]
    fn invalid_utf8_is_an_io_error_naming_the_file() {
        // 0xff starts no UTF-8 sequence, so the whole file is unusable.
        let err = Message::parse(b"Subject\n\xff broken\n", Path::new("COMMIT_EDITMSG"))
            .expect_err("invalid UTF-8 is an error");

        assert_eq!(err.exit_code(), 2, "spec §6: {err}");
        assert!(matches!(err, MessageError::NotUtf8 { .. }), "{err}");
        let message = err.to_string();
        assert!(message.contains("COMMIT_EDITMSG"), "{message}");
        assert!(message.contains("UTF-8"), "{message}");
    }

    #[test]
    fn invalid_utf8_in_the_first_byte_of_a_line_is_still_an_error() {
        let err = Message::parse(b"\xff# not a comment either", Path::new("msg.txt"))
            .expect_err("invalid UTF-8 is an error");

        assert_eq!(err.exit_code(), 2, "{err}");
        assert!(err.to_string().contains("msg.txt"), "{err}");
    }

    /// A scratch file under the system temp dir: no extra crate, and no test
    /// touches the real message file of a repository.
    fn scratch(name: &str, bytes: &[u8]) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cmac-message-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("creating the scratch directory");
        let path = dir.join(name);
        fs::write(&path, bytes).expect("writing the scratch file");
        path
    }

    #[test]
    fn read_parses_a_file_from_disk() {
        let path = scratch("msg.txt", b"Subject\r\n\r\nCo-authored-by: Claude\r\n");

        let message = Message::read(&path).expect("the file parses");
        let _ = fs::remove_dir_all(path.parent().expect("the scratch directory"));

        assert_eq!(
            message.subject().map(|line| line.text.as_str()),
            Some("Subject")
        );
        assert_eq!(
            message.lines[2].trailer.as_ref().map(|t| t.value.as_str()),
            Some("Claude")
        );
    }

    #[test]
    fn read_reports_invalid_utf8_with_the_file_name() {
        let path = scratch("bad.txt", b"\xff\xfe");

        let err = Message::read(&path).expect_err("invalid UTF-8 is an error");
        let _ = fs::remove_dir_all(path.parent().expect("the scratch directory"));

        assert_eq!(err.exit_code(), 2, "{err}");
        assert!(matches!(err, MessageError::NotUtf8 { .. }), "{err}");
        assert!(
            err.to_string().contains(&path.display().to_string()),
            "{err}"
        );
    }

    #[test]
    fn read_reports_a_missing_file_with_the_file_name() {
        let path = std::env::temp_dir().join("cmac-message-does-not-exist-msg.txt");

        let err = Message::read(&path).expect_err("a missing file is an error");

        assert_eq!(err.exit_code(), 2, "{err}");
        assert!(matches!(err, MessageError::Read { .. }), "{err}");
        assert!(
            err.to_string().contains(&path.display().to_string()),
            "{err}"
        );
    }
}
