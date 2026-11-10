//! Dogfooding (spec §2, §4.4): this repository's own commit prose, replayed
//! through `verify` with the built-in defaults.
//!
//! The point is false positives, not acceptance criteria. This project's
//! messages name Claude, Codex, Cursor and Amp while discussing the pattern
//! list, so a footer pattern that fires on prose would block the repo's own
//! commits. The historical messages are embedded below rather than read from
//! `git log`, so a rewritten history cannot quietly remove the evidence and a
//! fresh clone still runs the same fixtures.
//!
//! Each fixture is written to a scratch file and checked with no `--config`
//! and an `XDG_CONFIG_HOME` inside the scratch directory, so spec §3.2 step 4
//! applies the shipped defaults and the developer's own config file cannot
//! decide the result. Identity comes from the flags, so the machine's git
//! config cannot fire an identity pattern either.

mod support;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use support::{hook_script, shell_command};

const BIN: &str = env!("CARGO_BIN_EXE_commit-msg-attribution-checker");

/// Environment variable naming a config file (spec §3.1).
const ENV_CONFIG: &str = "COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG";

/// A plain human identity, so the `author` and `committer` targets cannot match
/// (spec §4.6) and the message is the only thing under test.
const HUMAN_IDENT: &[&str] = &[
    "--author",
    "Ada Lovelace <ada@example.com>",
    "--committer",
    "Ada Lovelace <ada@example.com>",
];

/// Commit `9267cc651c18903b6591e2697e04c3e72881b86b`, "Add v1 specification for
/// the checker", as git stored it (`git cat-file commit`, message part). The
/// spec commit the ticket names as the first dogfood fixture.
const SPEC_COMMIT: &str = "\
Add v1 specification for the checker

Defines the verify and config commands, the config resolution order, the
22 built-in patterns with their descriptions, the matching rules, and the
acceptance criteria.
";

/// Commit `495e3673cdd5c41373d5c6d0a28846061373fea8`, "Add backlog: 15 tickets
/// from the v1 spec". The backlog commit the ticket names as the second
/// fixture; its subject carries a colon-number prefix, which is trailer-shaped
/// but claimed by no configured trailer pattern (spec §5.2).
const BACKLOG_COMMIT: &str = "\
Add backlog: 15 tickets from the v1 spec

Decomposes the spec into a dependency DAG. The placeholder binary comes
first, the agent harness second, then config, parsing, and matching, and
finally the verify command with its hook integration.
";

/// A message that is legitimate prose for this project: the body states what
/// the spec table says, naming the tools it lists, which is not a footer.
const PROSE_NAMING_TOOLS: &str = "\
Renumber the pattern tables

The spec names Claude and Codex in the §4.1 table and the research doc lists
Cursor and Amp. Renumbering leaves the ids alone, so the matching is unchanged.
";

/// A plural that is not a tool name. The second line is footer-shaped on
/// purpose, so it is spec §4.4's word boundaries that keep `cursors` from
/// matching `cursor`, not the shape of the sentence.
const PROSE_PLURALS: &str = "\
Cope with amp in the parser

The parser holds two cursors, and a body line that says amp is prose.
Made with cursors, which is not the tool the registry names.
";

/// Prose that quotes the footer rules of spec §4.4: the clean example lines
/// appear inside a sentence, not at the start of a line.
const PROSE_FOOTER_RULES: &str = "\
Pin the footer rules in the spec

A line such as generated with an unknown tool stays clean, because the phrase
alone is not a marker. The spec keeps that gap on purpose.
";

/// Prose that quotes an indented trailer, which spec §5.2 gives no trailer
/// shape and no shipped pattern claims (§8 criterion 4).
const PROSE_INDENTED_TRAILER: &str = "\
Explain why an indented trailer is clean

  Co-authored-by: Claude <noreply@anthropic.com> is indented, so the anchored
expression gives it no trailer shape.
";

/// Prose about the provenance key names, which are trailer keys and only match
/// when the message carries the trailer (spec §4.3).
const PROSE_PROVENANCE_KEYS: &str = "\
Document the provenance keys

The claude-session key records a session id and the amp-thread-id key records a
link. Both are trailers, so naming them in a sentence is not attribution.
";

/// The project's real message shape: a subject, a body, and a `subtask:` trailer.
/// A trailer key that no configured pattern claims must not be rejected.
const PROJECT_SUBTASK_TRAILER: &str = "\
Add the dogfood fixtures

Replays the two historical messages through the built-in defaults.

subtask: dogfood
";

/// The probes above, as one list so a failure names the fixture.
const LEGITIMATE: &[(&str, &str)] = &[
    ("spec-commit", SPEC_COMMIT),
    ("backlog-commit", BACKLOG_COMMIT),
    ("prose-naming-tools", PROSE_NAMING_TOOLS),
    ("prose-plurals", PROSE_PLURALS),
    ("prose-footer-rules", PROSE_FOOTER_RULES),
    ("prose-indented-trailer", PROSE_INDENTED_TRAILER),
    ("prose-provenance-keys", PROSE_PROVENANCE_KEYS),
    ("project-subtask-trailer", PROJECT_SUBTASK_TRAILER),
];

/// A real attribution footer, to prove the checks below are not passing because
/// `verify` has stopped matching anything (spec §4.4).
const ATTRIBUTED: &str = "Add the parser\n\nGenerated with Claude Code\n";

/// The pattern the positive control must name (spec §4.4).
const FOOTER_PATTERN: &str = "free-form-footer.tool";

/// A line shaped like a denial. spec §4.4 requires a second signal and names a
/// known tool or model as one, so a line that starts with `Written by` and names
/// a tool matches by construction; the phrase alone does not. Pinned below as
/// the documented trade-off, not as a defect of this build.
const DENIAL_SENTENCE: &str = "\
Fix the build

Written by hand, with no Copilot involved.
";

/// A URL is the third second signal of spec §4.4, so a build note that carries
/// one at the start of a line matches whatever tool it names.
const BUILD_NOTE_WITH_URL: &str = "\
Fix the build

Made by Nix, per https://nixos.org/manual.
";

// The pattern the sentences above report (spec §4.4).
const FOOTER_TOOL_PATTERN: &str = "free-form-footer.tool";

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
            "cmac-dogfood-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("creating the scratch directory");
        Self { path }
    }

    fn join(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }

    fn message(&self, name: &str, bytes: &str) -> PathBuf {
        let path = self.join(&format!("msg-{name}.txt"));
        fs::write(&path, bytes).expect("writing the message file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// The environment every child gets: the config base points into the scratch
/// directory and the override is removed, so resolution of spec §3.2 can only
/// reach the built-in defaults.
fn isolate(command: &mut Command, scratch: &Scratch) {
    command
        .env("HOME", scratch.join("home"))
        .env("XDG_CONFIG_HOME", scratch.join("xdg"))
        .env_remove(ENV_CONFIG);
}

fn verify(scratch: &Scratch, message: &Path) -> Output {
    let mut command = Command::new(BIN);
    command
        .arg("verify")
        .arg(message)
        .args(HUMAN_IDENT)
        // Outside this repository on purpose: no identity target then depends
        // on the checkout the suite runs in.
        .current_dir(&scratch.path);
    isolate(&mut command, scratch);
    command.output().expect("running the binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

// spec §2 and §4.4 at the repository's own level: the messages this project
// writes name tools in prose, and every one of them is clean.
#[test]
fn this_repository_s_messages_and_prose_are_clean() {
    let scratch = Scratch::new("legitimate");

    for (name, text) in LEGITIMATE {
        let output = verify(&scratch, &scratch.message(name, text));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{name} must pass the shipped defaults; verify said:\n{}{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stdout.is_empty(),
            "{name}: a clean message writes nothing (spec §5.4): {}",
            stdout(&output)
        );
    }
}

// The same binary and the same config reject a footer that names a tool, so the
// clean verdicts above are a decision about these messages and not a check that
// has stopped matching (spec §5.3).
#[test]
fn the_probes_are_checked_against_the_live_pattern_list() {
    let scratch = Scratch::new("control");
    let output = verify(&scratch, &scratch.message("attributed", ATTRIBUTED));

    assert_eq!(
        output.status.code(),
        Some(1),
        "the positive control must be rejected: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout(&output).contains(FOOTER_PATTERN),
        "the control names the footer pattern: {}",
        stdout(&output)
    );
}

/// The hook this checkout installs, when it is installed.
///
/// `.git/hooks/` is untracked, so a fresh clone has no hook and the file's
/// absence is not a failure. When the file is there, it has to be the documented
/// script with the built binary resolved, because a hook that is stale or
/// mangled blocks the commits this project makes.
fn installed_hook() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let hook = root.join(".git").join("hooks").join("commit-msg");
    hook.is_file().then_some(hook)
}

// spec §7 against the real checkout: the installed hook is byte for byte what
// the README snippet becomes when the built binary is substituted.
#[test]
fn the_installed_hook_is_the_documented_snippet() {
    let Some(hook) = installed_hook() else {
        return;
    };
    let on_disk = fs::read_to_string(&hook).unwrap_or_else(|err| panic!("reading the hook: {err}"));

    assert_eq!(
        on_disk,
        hook_script(&shell_command(Path::new(BIN))),
        "{} must be spec §7's snippet with this build's binary resolved",
        hook.display()
    );
}

// The installed hook, run the way git runs it, on the fixtures above. This is
// the dogfood claim itself: the checkout's own gate accepts this project's
// commit prose.
#[test]
fn the_installed_hook_accepts_this_repository_s_messages() {
    let Some(_hook) = installed_hook() else {
        return;
    };
    let scratch = Scratch::new("hook");
    fs::write(
        scratch.join("gitconfig"),
        "[user]\n\tname = Ada Lovelace\n\temail = ada@example.com\n",
    )
    .expect("writing the scratch git config");

    for (name, text) in LEGITIMATE {
        let message = scratch.message(name, text);
        // `git hook run` starts the script with the `sh` git itself ships, so
        // the check does not depend on the host having one on PATH. The hook
        // passes the message path as `$1`, which is the argument after `--`.
        let mut command = Command::new("git");
        command
            .args(["hook", "run", "commit-msg", "--"])
            .arg(&message)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env("GIT_CONFIG_GLOBAL", scratch.join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1");
        isolate(&mut command, &scratch);
        let output = command.output().expect("running git hook run");

        assert_eq!(
            output.status.code(),
            Some(0),
            "{name}: the installed hook must accept this message; git said:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// The documented over-match of spec §4.4, pinned so a future edit to the footer
// patterns has to change this test on purpose. A line that begins with an
// attribution verb and names a tool matches even when the sentence denies it,
// because the second signal alone decides; the rules say nothing about
// negation. Weakening it is a spec §4 change, not a fix in the engine.
#[test]
fn a_denial_sentence_matches_on_its_second_signal() {
    let scratch = Scratch::new("denial");
    let output = verify(&scratch, &scratch.message("denial", DENIAL_SENTENCE));

    assert_eq!(
        output.status.code(),
        Some(1),
        "spec §4.4: phrase plus tool name is a match, whatever the sentence says\n{}",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains(FOOTER_TOOL_PATTERN),
        "the report names the footer pattern: {}",
        stdout(&output)
    );
}

// The same over-match through spec §4.4's third signal, so the pinned behaviour
// is the pattern's rule and not one alternation of it.
#[test]
fn a_build_note_with_a_url_matches_on_the_url_signal() {
    let scratch = Scratch::new("url");
    let output = verify(&scratch, &scratch.message("url", BUILD_NOTE_WITH_URL));

    assert_eq!(
        output.status.code(),
        Some(1),
        "spec §4.4: a URL after the phrase is the third second signal\n{}",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains("free-form-footer.url"),
        "the report names the url pattern: {}",
        stdout(&output)
    );
}
