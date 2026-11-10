//! End-to-end acceptance (spec §3.3, §8 criteria 9 and 11): the same message,
//! two config files, and the behaviour the file is supposed to decide.
//!
//! Criterion 9 is "a created config behaves like the built-ins"; criterion 11 is
//! "a user config disables every built-in it omits". Neither can be proven by
//! one command, so both cases run `config create` or a hand-written file, then
//! `config show` and `verify` on one message fixture. The fixture is a constant,
//! so the only variable between the cases is the config file.
//!
//! `config show` carries the second, independent pin on replacement semantics:
//! `verify`'s exit status could change for an unrelated reason, while the
//! effective pattern list can only lose `co-authored-by.claude` if the file
//! replaces the defaults instead of merging with them.
//!
//! Every child gets `XDG_CONFIG_HOME` and `HOME` inside a scratch directory and
//! has the config override removed, so no run reads or writes the machine's own
//! config path. Identity comes from `--author`/`--committer`, so no pattern can
//! fire on the invoking machine's git config and no test needs a repository.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_commit-msg-attribution-checker");

/// Environment variable naming a config file (spec §3.1).
const ENV_CONFIG: &str = "COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG";

/// A plain human identity, so the `author` and `committer` targets cannot match
/// and the message is the only thing under test (spec §4.6).
const HUMAN_IDENT: &[&str] = &[
    "--author",
    "Ada Lovelace <ada@example.com>",
    "--committer",
    "Ada Lovelace <ada@example.com>",
];

/// The one message both cases verify: criterion 9's message, a subject plus a
/// Claude co-author trailer.
const MESSAGE: &str = "Add the parser\n\nCo-authored-by: Claude <noreply@anthropic.com>\n";

/// The built-in pattern Case A must find and Case B must not have (spec §4.1).
const CLAUDE_PATTERN: &str = "co-authored-by.claude";

/// A user config whose single pattern cannot match the fixture: it claims the
/// trailer key `example-key`, which the message does not carry.
const ONE_UNRELATED_PATTERN: &str = "\
version = 1

[[pattern]]
id = \"example-key\"
description = \"Example-key trailer recording anything\"
target = \"trailer\"
key = \"example-key\"
regex = '\\S'
";

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
            "cmac-acceptance-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("creating the scratch directory");
        Self { path }
    }

    fn join(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }

    /// Write the shared fixture. Both cases call this, so the bytes are the
    /// constant above rather than each test's idea of the message.
    fn message(&self, name: &str) -> PathBuf {
        let path = self.join(&format!("msg-{name}.txt"));
        fs::write(&path, MESSAGE).expect("writing the message file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Run the binary with the config override cleared and the config base pointed
/// into the scratch directory, so resolution of spec §3.2 can only reach a file
/// the test named or wrote.
fn run(scratch: &Scratch, args: &[&str]) -> Output {
    let mut command = Command::new(BIN);
    command
        .args(args)
        .env("XDG_CONFIG_HOME", scratch.join("xdg"))
        .env("HOME", scratch.join("home"))
        .env_remove(ENV_CONFIG);
    command.output().expect("running the binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is utf-8")
}

/// `verify <message>` against an explicit config file, with the fixed ident.
fn verify(scratch: &Scratch, config: &Path, message: &Path) -> Output {
    let mut args: Vec<String> = vec![
        "--config".to_owned(),
        config.display().to_string(),
        "verify".to_owned(),
        message.display().to_string(),
    ];
    args.extend(HUMAN_IDENT.iter().map(|arg| (*arg).to_owned()));
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    run(scratch, &borrowed)
}

/// The pattern ids in the document `config show` prints, which is what `verify`
/// is about to apply (spec §6).
fn shown_pattern_ids(scratch: &Scratch, config: &Path) -> Vec<String> {
    let output = run(
        scratch,
        &["--config", &config.display().to_string(), "config", "show"],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "config show must succeed on this file: {}",
        stderr(&output)
    );

    let document = stdout(&output);
    let value: toml::Value = toml::from_str(&document)
        .unwrap_or_else(|err| panic!("config show prints TOML: {err}\n{document}"));
    value
        .get("pattern")
        .and_then(toml::Value::as_array)
        .map(|patterns| {
            patterns
                .iter()
                .map(|pattern| {
                    pattern
                        .get("id")
                        .and_then(toml::Value::as_str)
                        .unwrap_or_else(|| panic!("a pattern without an id: {pattern}"))
                        .to_owned()
                })
                .collect()
        })
        .unwrap_or_default()
}

// spec §8 criterion 9
#[test]
fn a_created_config_rejects_what_the_built_in_defaults_reject() {
    let scratch = Scratch::new("created");
    let message = scratch.message("created");
    let config = scratch.join("config.toml");
    let config_flag = config.display().to_string();

    let created = run(&scratch, &["--config", &config_flag, "config", "create"]);
    assert_eq!(
        created.status.code(),
        Some(0),
        "config create must succeed on a fresh path: {}",
        stderr(&created)
    );
    assert_eq!(
        stdout(&created).trim_end(),
        config.display().to_string(),
        "config create prints the path it wrote"
    );

    // spec §10: the written file is the default registry, not a second copy.
    let defaults = run(&scratch, &["config", "defaults"]);
    assert_eq!(defaults.status.code(), Some(0), "{}", stderr(&defaults));
    assert_eq!(
        fs::read_to_string(&config).expect("the file exists"),
        stdout(&defaults),
        "the created file is the built-in default set byte for byte"
    );

    let output = verify(&scratch, &config, &message);
    let block = stdout(&output);

    assert_eq!(
        output.status.code(),
        Some(1),
        "a created config behaves like the built-ins: {}",
        stderr(&output)
    );
    assert!(
        block.contains(&format!(
            "  {CLAUDE_PATTERN} (line 3, trailer co-authored-by)\n"
        )),
        "the trailer the fixture carries is reported: {block}"
    );

    let ids = shown_pattern_ids(&scratch, &config);
    assert!(
        ids.iter().any(|id| id == CLAUDE_PATTERN),
        "the effective config is the full default set: {ids:?}"
    );
}

// spec §8 criterion 11
#[test]
fn a_one_pattern_user_config_lets_the_claude_trailer_pass() {
    let scratch = Scratch::new("replaced");
    // The same fixture as criterion 9, so the config file is the only variable.
    let message = scratch.message("replaced");
    let config = scratch.join("other.toml");
    fs::write(&config, ONE_UNRELATED_PATTERN).expect("writing the user config");

    let output = verify(&scratch, &config, &message);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the file omits the Claude trailer, so nothing matches it: {}",
        stderr(&output)
    );
    assert!(
        output.stdout.is_empty(),
        "a clean message writes nothing (spec §5.4): {:?}",
        stdout(&output)
    );

    // spec §3.3, "a user file that lists one pattern disables every built-in
    // pattern that it omits": the effective list is exactly the one entry. This
    // assertion fails under merge semantics, which would show 23 patterns here.
    let ids = shown_pattern_ids(&scratch, &config);
    assert_eq!(
        ids,
        ["example-key"],
        "the file is the whole list, not an addition to the defaults"
    );
    assert!(
        !ids.iter().any(|id| id == CLAUDE_PATTERN),
        "spec §3.3: a built-in the file omits is disabled: {ids:?}"
    );
}
