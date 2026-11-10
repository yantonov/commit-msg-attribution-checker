//! `config show` contract (spec §6): the effective config on stdout, with one
//! header comment naming the source, exit 0, and exit 2 with the reason on
//! stderr for every load failure.
//!
//! Every test spawns the real binary, because what the command promises is the
//! bytes of the document `verify` will apply. Each child is given `--config` or
//! the environment variable, and its `XDG_CONFIG_HOME` and `HOME` point at a
//! scratch directory, so no run can reach the machine's own config path.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_commit-msg-attribution-checker");

/// Environment variable naming a config file (spec §3.1).
const ENV_CONFIG: &str = "COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG";

/// The default path below a config base (spec §3.1). The suffix is one literal,
/// matching how the binary joins it, so the test compares against the same
/// spelling the header prints.
fn default_path(base: &Path) -> PathBuf {
    base.join("commit-msg-attribution-checker/config.toml")
}

/// One pattern as the emitted document spells it.
struct Emitted {
    id: String,
    description: String,
    target: String,
    key: Option<String>,
    regex: String,
}

/// A scratch directory under the system temp dir: no extra crate, and no test
/// touches the real user profile.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("cmac-show-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).expect("creating the scratch directory");
        Self { path }
    }

    fn target(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }

    /// Write a file below the scratch directory and return its path.
    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.target(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("creating the parent directory");
        }
        fs::write(&path, text).expect("writing the file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Run the binary in the scratch directory with the isolation a test needs: no
/// inherited config override, and a private config base, so step 3 of spec §3.2
/// can only find a file the test wrote.
fn run(scratch: &Scratch, args: &[&str], env_config: Option<&Path>) -> Output {
    let mut command = Command::new(BIN);
    command.args(args);
    match env_config {
        Some(value) => command.env(ENV_CONFIG, value),
        None => command.env_remove(ENV_CONFIG),
    };
    // Both are set: `XDG_CONFIG_HOME` is what resolution reads first, and
    // `HOME` is the fallback if that variable is ever dropped on the way.
    command.env("XDG_CONFIG_HOME", scratch.target("xdg"));
    command.env("HOME", scratch.target("home"));
    command.output().expect("running the binary")
}

/// Assert a successful run and return stdout.
fn stdout_of(output: &Output) -> String {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "a successful command says nothing: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}

/// The single header line, asserting there is exactly one comment line and that
/// it comes first.
fn header(text: &str) -> String {
    let mut lines = text.lines();
    let first = lines
        .next()
        .unwrap_or_else(|| panic!("the document is empty"))
        .to_string();
    assert!(
        first.starts_with("# "),
        "the header is a comment: {first:?}"
    );
    for line in lines {
        assert!(
            !line.starts_with('#'),
            "only the header is a comment, found: {line:?}"
        );
    }
    first
}

/// The document with the header removed, so a test can compare it with
/// `config defaults`.
fn body(text: &str) -> String {
    text.lines().skip(1).collect::<Vec<_>>().join("\n")
}

/// Parse the printed body and read every pattern back out of it.
fn parse(text: &str) -> Vec<Emitted> {
    let document: toml::Value =
        toml::from_str(text).unwrap_or_else(|err| panic!("stdout is TOML: {err}\n{text}"));

    let table = |value: &toml::Value, field: &str, id: &str| -> String {
        value
            .get(field)
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("{id}: missing string field {field}"))
            .to_string()
    };

    let patterns = match document.get("pattern") {
        Some(value) => value
            .as_array()
            .unwrap_or_else(|| panic!("pattern is an array of tables: {value}"))
            .clone(),
        // A document with no `[[pattern]]` entry has no `pattern` key at all,
        // which is a valid config (spec §3.3).
        None => Vec::new(),
    };

    patterns
        .iter()
        .map(|value| {
            let id = table(value, "id", "a pattern");
            Emitted {
                description: table(value, "description", &id),
                target: table(value, "target", &id),
                key: value
                    .get("key")
                    .and_then(toml::Value::as_str)
                    .map(str::to_string),
                regex: table(value, "regex", &id),
                id,
            }
        })
        .collect()
}

fn ids(patterns: &[Emitted]) -> Vec<&str> {
    patterns.iter().map(|pattern| pattern.id.as_str()).collect()
}

const USER_CONFIG: &str = r#"
version = 1

[[pattern]]
id = "mine.claude"
description = "My own Claude rule"
target = "trailer"
key = "co-authored-by"
regex = '(?i)\bclaude\b'

[[pattern]]
id = "mine.subject"
description = "My own subject rule"
target = "subject"
regex = '^wip\b'
"#;

/// A config written by hand: odd spacing, single quotes, a trailing comment,
/// reordered fields and no `version`. Every one of those must survive a load and
/// disappear from the printed body.
const ODD_CONFIG: &str = r#"
   # my rules
[[pattern]]
      regex = '^aider'
   target = 'subject'   # not a trailer
 description    =   "Aider prefix"
			id = 'odd.one'

[[pattern]]
   id = "odd.two"
	key="assisted-by"
	target="trailer"
  regex='\S'
   description="Assisted-by"
"#;

#[test]
fn a_user_config_is_printed_with_its_path_in_the_header() {
    let scratch = Scratch::new();
    let config = scratch.write("config.toml", USER_CONFIG);

    let text = stdout_of(&run(
        &scratch,
        &["--config", config.to_str().unwrap(), "config", "show"],
        None,
    ));

    assert_eq!(
        header(&text),
        format!("# source: {}", config.display()),
        "spec §6: the header names the source path"
    );
    let patterns = parse(&body(&text));
    assert_eq!(ids(&patterns), ["mine.claude", "mine.subject"]);
    let trailer = &patterns[0];
    assert_eq!(trailer.target, "trailer");
    assert_eq!(trailer.key.as_deref(), Some("co-authored-by"));
    assert_eq!(trailer.regex, r"(?i)\bclaude\b");
    assert_eq!(trailer.description, "My own Claude rule");
    assert_eq!(patterns[1].key, None, "a subject pattern carries no key");
}

#[test]
fn the_environment_variable_config_is_printed_with_its_path() {
    let scratch = Scratch::new();
    let config = scratch.write("from-env.toml", USER_CONFIG);

    let text = stdout_of(&run(&scratch, &["config", "show"], Some(&config)));

    assert_eq!(header(&text), format!("# source: {}", config.display()));
    assert_eq!(ids(&parse(&body(&text))), ["mine.claude", "mine.subject"]);
}

#[test]
fn the_flag_wins_over_the_environment_variable() {
    let scratch = Scratch::new();
    let flag = scratch.write("from-flag.toml", USER_CONFIG);
    let env_config = scratch.write("from-env.toml", "version = 1\n");

    let text = stdout_of(&run(
        &scratch,
        &["--config", flag.to_str().unwrap(), "config", "show"],
        Some(&env_config),
    ));

    assert_eq!(header(&text), format!("# source: {}", flag.display()));
    assert_eq!(ids(&parse(&body(&text))), ["mine.claude", "mine.subject"]);
}

#[test]
fn the_file_at_the_default_path_is_printed_with_its_path() {
    // spec §3.2 step 3: no flag and no variable, so the default path decides.
    let scratch = Scratch::new();
    let config = default_path(&scratch.target("xdg"));
    fs::create_dir_all(config.parent().unwrap()).expect("creating the parent directory");
    fs::write(&config, USER_CONFIG).expect("writing the config");

    let text = stdout_of(&run(&scratch, &["config", "show"], None));

    assert_eq!(header(&text), format!("# source: {}", config.display()));
    assert_eq!(ids(&parse(&body(&text))), ["mine.claude", "mine.subject"]);
}

#[test]
fn with_no_user_config_the_defaults_apply_and_say_so() {
    // spec §3.2 step 4. The scratch `XDG_CONFIG_HOME` is empty, so the default
    // path cannot exist and this is the case the ticket names.
    let scratch = Scratch::new();
    assert!(!scratch.target("xdg").exists());

    let text = stdout_of(&run(&scratch, &["config", "show"], None));

    assert_eq!(header(&text), "# source: built-in defaults");
    let defaults = stdout_of(&run(&scratch, &["config", "defaults"], None));
    assert_eq!(
        body(&text),
        body(&format!("x\n{defaults}")),
        "spec §6: the fallback list is exactly what `config defaults` prints"
    );
    assert_eq!(
        parse(&body(&text)).len(),
        22,
        "spec §4: the full default set"
    );
}

#[test]
fn a_hand_edited_file_is_re_serialized_and_parses_back() {
    // The ticket's odd-formatting test: the body is the loaded model, not a copy
    // of the file text, and the printed document is still a config.
    let scratch = Scratch::new();
    let config = scratch.write("odd.toml", ODD_CONFIG);

    let text = stdout_of(&run(
        &scratch,
        &["--config", config.to_str().unwrap(), "config", "show"],
        None,
    ));
    let printed = body(&text);

    // Loads back through the binary's own model, which is what the ticket
    // checks with: the config crate is a dependency of this test binary too.
    let document: toml::Value =
        toml::from_str(&printed).unwrap_or_else(|err| panic!("{err}\n{printed}"));
    assert!(
        document.get("version").is_none(),
        "the file carried no version, so none is invented"
    );

    let patterns = parse(&printed);
    assert_eq!(ids(&patterns), ["odd.one", "odd.two"]);
    assert_eq!(patterns[0].description, "Aider prefix");
    assert_eq!(patterns[0].target, "subject");
    assert_eq!(patterns[0].regex, "^aider");
    assert_eq!(patterns[1].key.as_deref(), Some("assisted-by"));

    // The canonical spelling, not the file's: `id` first, then `description`.
    let first = &printed[printed.find("[[pattern]]").expect("a pattern table")..];
    let id = first.find("id = ").expect("id is emitted");
    let description = first
        .find("description = ")
        .expect("description is emitted");
    assert!(id < description, "{first}");

    assert!(
        !printed.contains('#'),
        "comments are not carried over: {printed}"
    );
    // Single-quoted values come back double-quoted, because the body is the
    // model re-serialized rather than the file text (one emitted regex keeps a
    // literal `'\S'`; that is a value, not quoting).
    for quoted in ["'^aider'", "'subject'", "'odd.one'"] {
        assert!(!printed.contains(quoted), "{quoted} survived: {printed}");
    }
}

#[test]
fn a_file_with_no_pattern_prints_the_header_only_and_parses() {
    // spec §3.3: no `[[pattern]]` entry is a valid file that disables all
    // checking, so the output must say nothing about patterns and still load.
    let scratch = Scratch::new();
    let config = scratch.write(
        "empty.toml",
        "# every pattern is commented out\nversion = 1\n",
    );

    let text = stdout_of(&run(
        &scratch,
        &["--config", config.to_str().unwrap(), "config", "show"],
        None,
    ));

    assert_eq!(header(&text), format!("# source: {}", config.display()));
    let printed = body(&text);
    assert!(!printed.contains("[[pattern]]"), "{printed}");
    assert!(printed.contains("version = 1"), "{printed}");
    assert!(parse(&printed).is_empty());

    // The binary itself accepts the printed document, which is the contract:
    // what `show` writes must be loadable by a later run.
    let printed_file = scratch.write("printed.toml", &text);
    let again = stdout_of(&run(
        &scratch,
        &["--config", printed_file.to_str().unwrap(), "config", "show"],
        None,
    ));
    assert_eq!(body(&again), printed, "the printed document round-trips");
}

#[test]
fn a_missing_explicit_config_is_exit_2_with_the_path_on_stderr() {
    // spec §3.2 step 1: a named path never falls back to the defaults.
    let scratch = Scratch::new();
    let absent = scratch.target("absent.toml");

    let output = run(
        &scratch,
        &["--config", absent.to_str().unwrap(), "config", "show"],
        None,
    );

    assert_eq!(
        output.status.code(),
        Some(2),
        "spec §6: a config error exits 2"
    );
    assert!(
        output.stdout.is_empty(),
        "nothing is printed as if it were the effective config: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr is utf-8");
    assert_eq!(
        stderr.matches(&absent.display().to_string()).count(),
        1,
        "the path is named once: {stderr}"
    );
    assert!(stderr.contains("config show"), "{stderr}");
}

#[test]
fn a_malformed_config_is_exit_2_and_names_the_reason() {
    let scratch = Scratch::new();
    let config = scratch.write(
        "broken.toml",
        "[[pattern]]\nid = \"broken\"\ndescription = \"d\"\ntarget = \"footer\"\nregex = \"x\"\n",
    );

    let output = run(
        &scratch,
        &["--config", config.to_str().unwrap(), "config", "show"],
        None,
    );

    assert_eq!(output.status.code(), Some(2), "spec §6");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr is utf-8");
    assert!(stderr.contains("config show"), "{stderr}");
    assert!(stderr.contains(&config.display().to_string()), "{stderr}");
    assert!(stderr.contains("footer"), "the reason is named: {stderr}");
}

#[test]
fn a_malformed_file_at_the_default_path_is_exit_2() {
    // spec §3.2 step 3 reads the default path, so a broken file there is an
    // error rather than a silent fallback to the defaults.
    let scratch = Scratch::new();
    let path = default_path(&scratch.target("xdg"));
    fs::create_dir_all(path.parent().unwrap()).expect("creating the parent directory");
    fs::write(&path, "not = [valid").expect("writing the broken config");

    let output = run(&scratch, &["config", "show"], None);

    assert_eq!(output.status.code(), Some(2), "spec §6");
    assert!(output.stdout.is_empty());
}
