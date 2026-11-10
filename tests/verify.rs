//! `verify` as the binary exposes it (spec §5.4, §6, §8): the report block on
//! stdout with exit 1, silence with exit 0, one stderr message with exit 2, and
//! nothing else leaving the process.
//!
//! Every test spawns the real binary in a scratch directory, with
//! `XDG_CONFIG_HOME`/`HOME` pointed at that directory and the config override
//! removed. A test that is about a shipped pattern passes no `--config`, so
//! spec §3.2 step 4 applies the built-ins; the tests that are about resolution
//! pass `--config` or the variable. No run reaches the machine's own config
//! path. Identity comes from `--author`/`--committer` except in the tests about
//! the repository probe, so no test depends on the git config of the machine it
//! runs on.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_commit-msg-attribution-checker");

/// Environment variable naming a config file (spec §3.1).
const ENV_CONFIG: &str = "COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG";

/// The two flags that make the identity independent of the machine, so a test
/// asserts the message half only.
const HUMAN_IDENT: &[&str] = &[
    "--author",
    "Ada Lovelace <ada@example.com>",
    "--committer",
    "Ada Lovelace <ada@example.com>",
];

/// A scratch directory under the system temp dir: no extra crate, and no test
/// touches the real user profile.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "cmac-verify-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("creating the scratch directory");
        Self { path }
    }

    fn join(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }

    /// A message file with these bytes, minus a name collision.
    fn message(&self, sender: &str, bytes: &str) -> PathBuf {
        let path = self.join(&format!("msg-{sender}.txt"));
        fs::write(&path, bytes).expect("writing the message file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Run the binary with the isolated environment a test needs.
///
/// `current_dir` is a parameter because the repository probe reads it: a test
/// about the identity skip must run outside every work tree.
fn run(
    dir: &Path,
    scratch: &Scratch,
    args: &[&str],
    env_config: Option<&Path>,
    path: Option<&Path>,
) -> Output {
    let mut command = Command::new(BIN);
    command
        .args(args)
        .current_dir(dir)
        .env("XDG_CONFIG_HOME", scratch.join("xdg"))
        .env("HOME", scratch.join("home"));
    match env_config {
        Some(value) => command.env(ENV_CONFIG, value),
        None => command.env_remove(ENV_CONFIG),
    };
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.output().expect("running the binary")
}

/// `verify <message>` plus the given extra arguments, as owned strings so a
/// scratch path outlives the call.
fn args(message: &Path, extra: &[&str]) -> Vec<String> {
    let mut args = vec!["verify".to_owned(), message.display().to_string()];
    args.extend(extra.iter().map(|arg| (*arg).to_owned()));
    args
}

fn run_verify(scratch: &Scratch, message: &Path, extra: &[&str]) -> Output {
    // No `--config`: the scratch `XDG_CONFIG_HOME` holds no file, so spec §3.2
    // step 4 applies the shipped patterns. A user file would replace them
    // (spec §3.3), and a test about a pattern needs the registry that ships it.
    let owned = args(message, extra);
    let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();
    run(&scratch.path, scratch, &borrowed, None, None)
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is utf-8")
}

/// Assert a clean run: exit 0 and not one byte on stdout (spec §5.4).
fn assert_clean(output: &Output) {
    let notes = stderr(output);
    assert_eq!(output.status.code(), Some(0), "stderr: {notes}");
    assert!(
        output.stdout.is_empty(),
        "a clean message writes nothing, not even a newline: {:?}",
        stdout(output)
    );
}

/// The number of stanzas in a block: the lines indented two spaces and no more.
fn stanzas(block: &str) -> usize {
    block
        .lines()
        .filter(|line| line.starts_with("  ") && !line.starts_with("    "))
        .count()
}

fn require_git() {
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => panic!(
            "these tests need git: `git --version` exited {:?}",
            output.status
        ),
        Err(err) => panic!("these tests need a git binary on PATH: {err}"),
    }
}

/// A scratch directory no repository claims, with the assumption checked rather
/// than assumed: the repo under the user's home makes "a temp dir is outside"
/// worth asserting.
fn outside_a_repository(name: &str) -> Scratch {
    require_git();
    let scratch = Scratch::new(name);
    let probe = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(&scratch.path)
        .output()
        .expect("running the probe");
    assert!(
        !probe.status.success(),
        "the fixture must be outside a repository; {} is inside one, so point \
         TMPDIR somewhere git does not claim",
        scratch.path.display()
    );
    scratch
}

// spec §8 criterion 1
#[test]
fn a_claude_co_author_trailer_reports_and_exits_1() {
    let scratch = Scratch::new("claude");
    let message = scratch.message(
        "claude",
        "Add the parser\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
    );

    let output = run_verify(&scratch, &message, HUMAN_IDENT);
    let block = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert!(
        block.starts_with("commit-msg-attribution-checker: attribution detected\n\n"),
        "{block:?}"
    );
    assert!(
        block.contains("  co-authored-by.claude (line 3, trailer co-authored-by)\n"),
        "{block}"
    );
    assert!(
        block.contains("    Co-authored-by trailer naming Claude\n"),
        "{block}"
    );
    assert!(
        block.contains("    value: Claude <noreply@anthropic.com>\n"),
        "{block}"
    );
    assert!(
        block.ends_with("\n\n1 match. Commit rejected.\n"),
        "one match closes with the singular: {block:?}"
    );
    assert_eq!(stanzas(&block), 1, "{block}");
    assert!(
        stderr(&output).is_empty(),
        "a report is not a diagnostic: {}",
        stderr(&output)
    );
}

// spec §8 criterion 2
#[test]
fn a_human_co_author_is_clean() {
    let scratch = Scratch::new("human");
    let message = scratch.message(
        "human",
        "Add the parser\n\nCo-authored-by: Alice <alice@example.com>\n",
    );

    assert_clean(&run_verify(&scratch, &message, HUMAN_IDENT));
}

// spec §8 criterion 3
#[test]
fn three_attribution_lines_report_three_matches() {
    let scratch = Scratch::new("three");
    let message = scratch.message(
        "three",
        "Add the parser\n\n\
         Co-authored-by: Claude <noreply@anthropic.com>\n\
         Made-with: Cursor\n\
         \u{1f916} Generated with Amp\n",
    );

    let output = run_verify(&scratch, &message, HUMAN_IDENT);
    let block = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert_eq!(stanzas(&block), 3, "{block}");
    for id in [
        "co-authored-by.claude",
        "made-with",
        "free-form-footer.emoji",
    ] {
        assert!(block.contains(&format!("  {id} (line ")), "{id}: {block}");
    }
    assert!(
        block.ends_with("\n\n3 matches. Commit rejected.\n"),
        "the count is the match count: {block:?}"
    );
}

// spec §8 criterion 4, first half: a `#` in the first column is not a message
// line, so the trailer it shows cannot match.
#[test]
fn a_comment_line_is_clean() {
    let scratch = Scratch::new("comment");
    let commented = scratch.message(
        "commented",
        "Add the parser

# Co-authored-by: Claude <noreply@anthropic.com>
",
    );

    assert_clean(&run_verify(&scratch, &commented, HUMAN_IDENT));
}

// spec §8 criterion 4, second half: an indented attribution is not a comment
// line (§5.1), but §5.2's trailer expression is anchored at the line start, so
// it has no trailer shape and no shipped pattern claims it. The line is clean.
#[test]
fn an_indented_co_author_trailer_is_clean() {
    let scratch = Scratch::new("indented");
    let indented = scratch.message(
        "indented",
        "Add the parser

  # Co-authored-by: Claude <noreply@anthropic.com>
",
    );

    assert_clean(&run_verify(&scratch, &indented, HUMAN_IDENT));
}

// The same leading spaces without the `#`, so the case above is read as what it
// is: the trailer shape is what decides, and a leading space is enough to lose
// it. §5.1's comment rule never has to be invoked for this line.
#[test]
fn an_indented_co_author_trailer_carries_no_trailer_shape() {
    let scratch = Scratch::new("indented-trailer");
    let indented = scratch.message(
        "indented-trailer",
        "Add the parser

  Co-authored-by: Claude <noreply@anthropic.com>
",
    );

    assert_clean(&run_verify(&scratch, &indented, HUMAN_IDENT));
}

// spec §8 criterion 5
#[test]
fn prose_about_a_build_is_clean_and_the_two_attribution_footers_report() {
    let scratch = Scratch::new("footer");
    let prose = scratch.message(
        "prose",
        "Add the parser\n\nBody\n\ngenerated by the build script\n",
    );

    assert_clean(&run_verify(&scratch, &prose, HUMAN_IDENT));

    for line in [
        "Made with Cursor",
        "\u{1f916} Generated with something unknown",
    ] {
        let message = scratch.message("footer", &format!("Add the parser\n\n{line}\n"));
        let output = run_verify(&scratch, &message, HUMAN_IDENT);
        let block = stdout(&output);

        assert_eq!(output.status.code(), Some(1), "{line}: {}", stderr(&output));
        assert!(block.contains("free-form-footer."), "{line}: {block}");
    }
}

// spec §8 criterion 6, asserted through the binary as a regression on the
// engine's report: the trailer gate must not let the footer patterns report the
// same attribution a second time.
#[test]
fn an_assisted_by_trailer_reports_exactly_one_match() {
    let scratch = Scratch::new("assisted-by");
    let message = scratch.message(
        "assisted-by",
        "Add the parser\n\nAssisted-by: Amp <amp@ampcode.com>\n",
    );

    let output = run_verify(&scratch, &message, HUMAN_IDENT);
    let block = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert_eq!(stanzas(&block), 1, "one attribution, one stanza: {block}");
    assert!(
        block.contains("  assisted-by (line 3, trailer assisted-by)\n"),
        "{block}"
    );
    assert!(
        !block.contains("free-form-footer"),
        "the line is claimed by the trailer pattern: {block}"
    );
    assert!(
        block.ends_with("\n\n1 match. Commit rejected.\n"),
        "{block:?}"
    );
}

// spec §8 criterion 7
#[test]
fn an_aider_subject_reports_the_subject_pattern() {
    let scratch = Scratch::new("aider-subject");
    let message = scratch.message("aider-subject", "aider: fix validation\n");

    let output = run_verify(&scratch, &message, HUMAN_IDENT);
    let block = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert!(
        block.contains("  aider.subject (line 1, subject)\n"),
        "{block}"
    );
    assert!(
        block.contains("    value: aider: fix validation\n"),
        "{block}"
    );
}

// spec §8 criterion 8
#[test]
fn an_empty_message_and_a_comment_only_message_are_clean() {
    let scratch = Scratch::new("empty");

    for (name, bytes) in [
        ("empty", ""),
        (
            "comment-only",
            "# Please enter the commit message\n#\n# On branch master\n",
        ),
    ] {
        let message = scratch.message(name, bytes);
        assert_clean(&run_verify(&scratch, &message, HUMAN_IDENT));
    }
}

// spec §8 criterion 12
#[test]
fn outside_a_repository_a_clean_message_exits_0() {
    let scratch = outside_a_repository("outside-clean");
    let message = scratch.message("outside-clean", "Add the parser\n\nBody\n");

    // No identity flags: this is the run that must not fail outside a repo.
    assert_clean(&run_verify(&scratch, &message, &[]));
}

// spec §8 criterion 12
#[test]
fn outside_a_repository_a_matching_message_exits_1() {
    let scratch = outside_a_repository("outside-match");
    let message = scratch.message(
        "outside-match",
        "Add the parser\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
    );

    let output = run_verify(&scratch, &message, &[]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "the exit status follows the message targets alone: {}",
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("co-authored-by.claude"),
        "{:?}",
        stdout(&output)
    );
}

// spec §8 criterion 12
#[test]
fn outside_a_repository_v_names_both_skipped_targets_on_stderr() {
    let scratch = outside_a_repository("outside-verbose");
    let message = scratch.message("outside-verbose", "Add the parser\n");

    let quiet = run_verify(&scratch, &message, &[]);
    let verbose = run_verify(&scratch, &message, &["-v"]);

    assert_clean(&quiet);
    assert_clean(&verbose);
    let notes = stderr(&verbose);
    assert!(notes.contains("skipping author"), "{notes}");
    assert!(notes.contains("skipping committer"), "{notes}");
    assert!(notes.contains("not inside a repository"), "{notes}");
    assert!(
        quiet.stdout == verbose.stdout,
        "stdout is byte-identical with and without -v"
    );
}

// The criterion 12 half that a match makes checkable: `-v` must not touch the
// report, because a hook log and a CI log show different streams.
#[test]
fn v_never_changes_stdout() {
    let scratch = outside_a_repository("verbose-match");
    let message = scratch.message(
        "verbose-match",
        "aider: fix validation\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
    );

    let quiet = run_verify(&scratch, &message, &[]);
    let verbose = run_verify(&scratch, &message, &["-v"]);

    assert_eq!(quiet.status.code(), Some(1), "{}", stderr(&quiet));
    assert_eq!(verbose.status.code(), Some(1), "{}", stderr(&verbose));
    assert_eq!(
        quiet.stdout, verbose.stdout,
        "spec §5.4: the report is the report, with or without -v"
    );
    assert!(stderr(&quiet).is_empty(), "{}", stderr(&quiet));
    assert!(
        stderr(&verbose).contains("skipping"),
        "{}",
        stderr(&verbose)
    );
}

#[test]
fn a_missing_message_file_is_exit_2_and_names_the_path() {
    let scratch = Scratch::new("missing-message");
    let absent = scratch.join("does-not-exist.txt");

    let output = run_verify(&scratch, &absent, HUMAN_IDENT);
    let notes = stderr(&output);

    assert_eq!(output.status.code(), Some(2), "{notes}");
    assert!(
        output.stdout.is_empty(),
        "an error writes nothing to stdout: {:?}",
        stdout(&output)
    );
    assert!(notes.contains(&absent.display().to_string()), "{notes}");
    assert!(
        notes.starts_with("commit-msg-attribution-checker: verify: "),
        "{notes}"
    );
}

#[test]
fn a_config_error_is_exit_2_and_outranks_the_identity() {
    // The message and the flags are fine, so the only failure is the config.
    let scratch = Scratch::new("bad-config");
    let message = scratch.message("bad-config", "Add the parser\n");
    let config = scratch.join("broken.toml");
    fs::write(&config, "not = [valid").expect("writing the broken config");

    let config_flag = config.display().to_string();
    let mut owned = vec!["--config".to_owned(), config_flag];
    owned.extend(args(&message, HUMAN_IDENT));
    let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();

    let output = run(&scratch.path, &scratch, &borrowed, None, None);
    let notes = stderr(&output);

    assert_eq!(output.status.code(), Some(2), "{notes}");
    assert!(output.stdout.is_empty(), "{:?}", stdout(&output));
    assert!(notes.contains(&config.display().to_string()), "{notes}");
    assert!(
        notes.starts_with("commit-msg-attribution-checker: verify: "),
        "{notes}"
    );
}

#[test]
fn a_broken_config_and_a_missing_message_name_the_message_first() {
    // The pinned order: a bad message path is the most common user error and a
    // file read is cheap, so the message is reported even when both are wrong.
    let scratch = Scratch::new("both-broken");
    let absent = scratch.join("does-not-exist.txt");
    let config = scratch.join("broken.toml");
    fs::write(&config, "not = [valid").expect("writing the broken config");

    let config_flag = config.display().to_string();
    let mut owned = vec!["--config".to_owned(), config_flag];
    owned.extend(args(&absent, HUMAN_IDENT));
    let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();

    let output = run(&scratch.path, &scratch, &borrowed, None, None);
    let notes = stderr(&output);

    assert_eq!(output.status.code(), Some(2), "{notes}");
    assert!(output.stdout.is_empty());
    assert!(notes.contains(&absent.display().to_string()), "{notes}");
    assert!(
        !notes.contains(&config.display().to_string()),
        "one message, and it is the message file's: {notes}"
    );
    assert_eq!(
        notes.lines().count(),
        1,
        "an error prints one message: {notes}"
    );
}

#[test]
fn an_unreadable_message_file_is_exit_2() {
    // A directory is read-able as a path but not as a message, so this is the
    // I/O error a permission problem would also produce.
    let scratch = Scratch::new("unreadable");
    let dir = scratch.join("a-directory");
    fs::create_dir_all(&dir).expect("creating the directory");

    let output = run_verify(&scratch, &dir, HUMAN_IDENT);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert!(
        stderr(&output).contains(&dir.display().to_string()),
        "{}",
        stderr(&output)
    );
}

/// Strip the one trailing newline `render_json` adds and the JSON text, so a
/// test can assert the document without restating that it ends a line.
fn json_body(output: &Output) -> String {
    let raw = stdout(output);
    let body = raw
        .strip_suffix('\n')
        .unwrap_or_else(|| panic!("the JSON document ends with a newline: {raw:?}"));
    body.to_owned()
}

/// Assert the one match-document shape a match must have: a top-level object,
/// one entry, and the five keys spec §5.3 lists, in that order.
fn assert_one_match_document(document: &str) {
    assert!(
        document.starts_with("{\"matches\":["),
        "a top-level object, not a bare array: {document}"
    );
    assert!(document.ends_with("]}"), "{document}");
    assert_eq!(
        document.matches("\"id\":").count(),
        1,
        "one object per match: {document}"
    );
    let keys = [
        "\"id\":",
        "\"description\":",
        "\"target\":",
        "\"value\":",
        "\"line\":",
    ];
    let mut cursor = 0;
    for key in keys {
        let at = document[cursor..]
            .find(key)
            .unwrap_or_else(|| panic!("{key} is missing: {document}"))
            + cursor;
        assert!(at >= cursor, "{key} is out of order: {document}");
        cursor = at + key.len();
    }
}

#[test]
fn json_writes_the_match_as_a_document_and_exits_1() {
    // spec §5.4: `--json` writes the matches as JSON. This is the flag that
    // used to be refused, so the test is the same fixture with the new outcome.
    let scratch = Scratch::new("json");
    let message = scratch.message(
        "json",
        "Add the parser\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
    );

    let output = run_verify(&scratch, &message, &["--json"]);
    let document = json_body(&output);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert_one_match_document(&document);
    assert!(
        document.contains("\"id\":\"co-authored-by.claude\""),
        "{document}"
    );
    assert!(
        document.contains("\"description\":\"Co-authored-by trailer naming Claude\""),
        "{document}"
    );
    assert!(document.contains("\"target\":\"trailer\""), "{document}");
    assert!(
        document.contains("\"value\":\"Claude <noreply@anthropic.com>\""),
        "{document}"
    );
    assert!(
        document.contains("\"line\":3}"),
        "the message line number, as a number: {document}"
    );
    assert!(
        !stdout(&output).contains("attribution detected"),
        "the flag replaces the block, it does not add to it: {document}"
    );
    assert!(
        stderr(&output).is_empty(),
        "a document is not a diagnostic: {}",
        stderr(&output)
    );
}

#[test]
fn json_reports_one_object_per_match() {
    let scratch = Scratch::new("json-three");
    let message = scratch.message(
        "json-three",
        "Add the parser\n\n\
         Co-authored-by: Claude <noreply@anthropic.com>\n\
         Made-with: Cursor\n\
         \u{1f916} Generated with Amp\n",
    );

    let output = run_verify(&scratch, &message, &["--json"]);
    let document = json_body(&output);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert_eq!(
        document.matches("\"id\":").count(),
        3,
        "three matches, three objects: {document}"
    );
    for id in [
        "co-authored-by.claude",
        "made-with",
        "free-form-footer.emoji",
    ] {
        assert!(
            document.contains(&format!("\"id\":\"{id}\"")),
            "{id}: {document}"
        );
    }
    // spec §5.3's line numbers, so the document is the report and not a summary.
    for line in ["\"line\":3}", "\"line\":4}", "\"line\":5}"] {
        assert!(document.contains(line), "{line}: {document}");
    }
}

#[test]
fn json_and_the_human_block_agree_on_the_exit_status() {
    // spec §5.4: "Exit codes do not change". Both formats over the same bytes.
    let scratch = Scratch::new("json-status");
    let ident = [
        "--author",
        "Ada Lovelace <ada@example.com>",
        "--committer",
        "Ada Lovelace <ada@example.com>",
    ];
    for (name, bytes, expected) in [
        ("clean", "Add the parser\n\nBody\n", Some(0)),
        (
            "match",
            "Add the parser\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
            Some(1),
        ),
    ] {
        let message = scratch.message(name, bytes);
        let human = run_verify(&scratch, &message, &ident);
        let mut json_args = vec!["--json"];
        json_args.extend(ident);
        let json = run_verify(&scratch, &message, &json_args);

        assert_eq!(human.status.code(), expected, "{name}: {}", stderr(&human));
        assert_eq!(
            json.status.code(),
            human.status.code(),
            "{name}: the flag changes the format, not the status: {}",
            stderr(&json)
        );
    }
}

#[test]
fn json_of_a_clean_message_is_an_empty_document_and_exits_0() {
    // The deliberate divergence from the human path, which writes nothing at
    // all: an empty `matches` array is how a CI consumer tells "found nothing"
    // from "did not run".
    let scratch = Scratch::new("json-clean");
    let message = scratch.message("json-clean", "Add the parser\n\nBody\n");

    let output = run_verify(&scratch, &message, &["--json", "-v"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert_eq!(
        json_body(&output),
        "{\"matches\":[]}",
        "valid JSON, and not silence"
    );
}

#[test]
fn json_v_keeps_the_skip_notes_off_stdout() {
    // spec §6: every diagnostic goes to stderr, so `-v` cannot corrupt the
    // document a consumer parses.
    let scratch = outside_a_repository("json-verbose");
    let message = scratch.message(
        "json-verbose",
        "Add the parser\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
    );

    let output = run_verify(&scratch, &message, &["--json", "-v"]);
    let document = json_body(&output);
    let notes = stderr(&output);

    assert_eq!(output.status.code(), Some(1), "{notes}");
    assert!(
        json_body(&output).starts_with("{\"matches\":["),
        "stdout is the document alone: {document}"
    );
    assert!(!document.contains("skipping"), "{document}");
    assert!(notes.contains("skipping"), "{notes}");
}

#[test]
fn json_writes_nothing_to_stdout_when_the_message_file_is_missing() {
    // Done when: "An error path writes nothing to stdout, so JSON consumers
    // never see a partial document."
    let scratch = Scratch::new("json-missing-message");
    let absent = scratch.join("does-not-exist.txt");

    let output = run_verify(&scratch, &absent, &["--json"]);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(
        output.stdout.is_empty(),
        "zero bytes, not even `{{\"matches\":[]}}`: {:?}",
        stdout(&output)
    );
    assert!(
        stderr(&output).contains(&absent.display().to_string()),
        "{}",
        stderr(&output)
    );
}

#[test]
fn json_writes_nothing_to_stdout_when_the_config_is_broken() {
    let scratch = Scratch::new("json-bad-config");
    let message = scratch.message("json-bad-config", "Add the parser\n");
    let config = scratch.join("broken.toml");
    fs::write(&config, "not = [valid").expect("writing the broken config");

    let config_flag = config.display().to_string();
    let mut owned = vec!["--config".to_owned(), config_flag];
    owned.extend(args(&message, &["--json"]));
    let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();

    let output = run(&scratch.path, &scratch, &borrowed, None, None);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(
        output.stdout.is_empty(),
        "an error is not an empty match list: {:?}",
        stdout(&output)
    );
    assert!(
        stderr(&output).contains(&config.display().to_string()),
        "{}",
        stderr(&output)
    );
}

#[test]
fn json_of_an_identity_match_carries_a_null_line() {
    // spec §5.3: an identity has no message line, and the key stays present so a
    // consumer can rely on a stable set.
    let scratch = Scratch::new("json-identity");
    let message = scratch.message("json-identity", "Add the parser\n");

    let output = run_verify(
        &scratch,
        &message,
        &[
            "--json",
            "--author",
            "Cursor <dev@cursor.com>",
            "--committer",
            "Ada Lovelace <ada@example.com>",
        ],
    );
    let document = json_body(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(document.contains("\"target\":\"author\""), "{document}");
    assert!(document.contains("\"line\":null}"), "{document}");
}

#[test]
fn json_escapes_a_value_that_carries_json_metacharacters() {
    // A whole line is a value, so a quote or a backslash in the message reaches
    // the emitter. The document must stay parseable: the raw characters never
    // appear inside a string.
    let scratch = Scratch::new("json-escapes");
    // The line names Claude, so `free-form-footer.tool` reports it whole.
    let message = scratch.message(
        "json-escapes",
        "Add the parser\n\nGenerated with Claude \"quoted\" \\ backslash\n",
    );

    let output = run_verify(&scratch, &message, &["--json"]);
    let document = json_body(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        document.contains("\"value\":\"Generated with Claude \\\"quoted\\\" \\\\ backslash\""),
        "{document}"
    );
    // Crude but exact for this fixture: the document holds no raw quote inside a
    // value, and the counts of the escaped forms are what the input had.
    assert_eq!(document.matches("\\\"").count(), 2, "{document}");
    assert_eq!(document.matches("\\\\").count(), 1, "{document}");
}

#[test]
fn the_environment_variable_config_is_used() {
    // spec §3.2 step 2, through `verify`: no `--config`, so the environment
    // variable decides, and a list with one pattern lets Claude through.
    let scratch = Scratch::new("env-config");
    let message = scratch.message(
        "env-config",
        "Add the parser\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
    );
    let config = scratch.join("one.toml");
    fs::write(
        &config,
        "version = 1\n\n[[pattern]]\nid = \"mine\"\ndescription = \"Mine\"\n\
         target = \"subject\"\nregex = '^wip'\n",
    )
    .expect("writing the config");

    let owned = args(&message, HUMAN_IDENT);
    let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();
    let output = run(&scratch.path, &scratch, &borrowed, Some(&config), None);

    assert_clean(&output);
}

#[test]
fn the_defaults_apply_when_no_config_file_exists() {
    // spec §3.2 step 4, and the path every test above takes: `XDG_CONFIG_HOME`
    // points at an empty scratch directory, so no file can be found and the
    // shipped patterns decide. This is what makes the tests above independent of
    // the machine's own config.
    let scratch = Scratch::new("defaults");
    let message = scratch.message(
        "defaults",
        "Add the parser\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
    );

    let owned = args(&message, HUMAN_IDENT);
    let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();
    let output = run(&scratch.path, &scratch, &borrowed, None, None);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("co-authored-by.claude"),
        "{:?}",
        stdout(&output)
    );
}

#[test]
fn a_config_flag_list_is_the_whole_list() {
    // spec §3.3: the file replaces the shipped patterns. A one-pattern file that
    // says nothing about Claude therefore lets the trailer through, which is
    // also how a test knows `--config` reached the command.
    let scratch = Scratch::new("flag-config");
    let message = scratch.message(
        "flag-config",
        "Add the parser\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
    );
    let config = scratch.join("one.toml");
    fs::write(
        &config,
        "version = 1\n\n[[pattern]]\nid = \"mine\"\ndescription = \"Mine\"\n\
         target = \"subject\"\nregex = '^wip'\n",
    )
    .expect("writing the config");

    let config_flag = config.display().to_string();
    let mut owned = vec!["--config".to_owned(), config_flag];
    owned.extend(args(&message, HUMAN_IDENT));
    let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();

    assert_clean(&run(&scratch.path, &scratch, &borrowed, None, None));
}

#[test]
fn the_author_flag_is_what_the_author_target_tests() {
    // spec §5.2: the flag overrides `git var`. Running inside the scratch dir
    // with both flags means no git call happens at all.
    let scratch = Scratch::new("author-flag");
    let message = scratch.message("author-flag", "Add the parser\n");

    let output = run_verify(
        &scratch,
        &message,
        &[
            "--author",
            "Cursor <dev@cursor.com>",
            "--committer",
            "Ada Lovelace <ada@example.com>",
        ],
    );
    let block = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        block.contains("  author.agent-identity (author)\n"),
        "an identity match has no message line: {block}"
    );
    assert!(
        block.contains("    value: Cursor <dev@cursor.com>\n"),
        "{block}"
    );
}
