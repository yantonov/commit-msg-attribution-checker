//! `config defaults` contract (spec §6): the built-in config as TOML on stdout,
//! exit 0, and no dependence on the user config.
//!
//! Every test spawns the real binary, because what the command promises is
//! bytes on stdout and an exit status, not a function's return value.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_commit-msg-attribution-checker");

/// Environment variable naming a config file (spec §3.1).
const ENV_CONFIG: &str = "COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG";

/// One pattern as the emitted file spells it, so a test states what it expects
/// to read rather than re-deriving it.
struct Emitted {
    id: String,
    target: String,
    key: Option<String>,
    regex: String,
    description: String,
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
        let path = env::temp_dir().join(format!("cmac-defaults-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).expect("creating the scratch directory");
        Self { path }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run(args: &[&str], env_config: Option<&str>) -> Output {
    let mut command = Command::new(BIN);
    command.args(args);
    if let Some(value) = env_config {
        command.env(ENV_CONFIG, value);
    } else {
        // The machine's own override would otherwise decide what the binary
        // reads, so every test states this input explicitly.
        command.env_remove(ENV_CONFIG);
    }
    command.output().expect("running the binary")
}

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

/// Parse the emitted document and read every pattern back out of it.
fn parse(text: &str) -> Vec<Emitted> {
    let document: toml::Value =
        toml::from_str(text).unwrap_or_else(|err| panic!("stdout is TOML: {err}\n{text}"));

    assert_eq!(
        document.get("version").and_then(toml::Value::as_integer),
        Some(1),
        "spec §3.4: the file leads with version = 1"
    );

    let table = |value: &toml::Value, field: &str, id: &str| -> String {
        value
            .get(field)
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("{id}: missing string field {field}"))
            .to_string()
    };

    document
        .get("pattern")
        .and_then(toml::Value::as_array)
        .expect("an array of [[pattern]] tables")
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

#[test]
fn writes_the_builtin_config_to_stdout_and_exits_zero() {
    let output = run(&["config", "defaults"], None);
    let patterns = parse(&stdout_of(&output));

    assert_eq!(
        patterns.len(),
        22,
        "spec §4: every built-in pattern is printed"
    );

    let ids: Vec<&str> = patterns.iter().map(|pattern| pattern.id.as_str()).collect();
    assert_eq!(
        ids.first().copied(),
        Some("co-authored-by.claude"),
        "spec order is kept"
    );
    assert_eq!(
        ids.last().copied(),
        Some("committer.agent-identity"),
        "spec order is kept"
    );

    let unique: BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len(), "spec §3.4: ids are unique");

    for pattern in &patterns {
        assert!(
            !pattern.description.trim().is_empty(),
            "{}: a pattern without a description names no reason",
            pattern.id
        );
        assert!(
            !pattern.regex.is_empty() && !pattern.regex.contains('\n'),
            "{}: the regex is written on one line",
            pattern.id
        );
        // The pairing the config model enforces on load (spec §3.4): reading it
        // here names the pattern instead of a parse error later.
        assert_eq!(
            pattern.target == "trailer",
            pattern.key.is_some(),
            "{}: only trailer patterns carry a key",
            pattern.id
        );
    }
}

#[test]
fn output_carries_no_header_comment() {
    // The header comment is what `config show` adds to name the source path
    // (spec §6). This command's source is the binary, so a `#` line here would
    // mean the two commands had collapsed into one.
    let text = stdout_of(&run(&["config", "defaults"], None));

    assert_eq!(
        text.lines().next(),
        Some("version = 1"),
        "the first line is the document, not a comment:\n{text}"
    );
    for line in text.lines() {
        assert!(
            !line.starts_with('#'),
            "no comment line is emitted, found: {line:?}"
        );
    }
}

#[test]
fn output_is_byte_identical_with_and_without_a_config_path() {
    // The identity half this ticket can prove: the command takes its bytes from
    // one serializer, so an accepted-but-unused `--config` cannot change them.
    // The matching assertion against `config create` belongs to that command.
    let scratch = Scratch::new();
    let config = scratch.path.join("config.toml");
    fs::write(
        &config,
        "[[pattern]]\nid = \"only-this\"\ndescription = \"d\"\ntarget = \"line\"\nregex = \"x\"\n",
    )
    .expect("writing the user config");
    let config = config.to_str().expect("a utf-8 path");

    let plain = stdout_of(&run(&["config", "defaults"], None));
    let with_flag = stdout_of(&run(&["--config", config, "config", "defaults"], None));

    assert_eq!(plain, with_flag);
    assert_eq!(parse(&plain).len(), 22, "the defaults win over the file");
}

#[test]
fn a_missing_config_path_is_ignored() {
    // If the command resolved the config, this path would be exit 2 (spec §3.2).
    let scratch = Scratch::new();
    let absent = scratch.path.join("absent.toml");
    let absent = absent.to_str().expect("a utf-8 path");

    let output = run(&["--config", absent, "config", "defaults"], Some(absent));
    let text = stdout_of(&output);

    assert_eq!(parse(&text).len(), 22);
    assert_eq!(text, stdout_of(&run(&["config", "defaults"], None)));
}

#[test]
fn a_missing_config_environment_variable_path_is_ignored() {
    let scratch = Scratch::new();
    let absent = scratch.path.join("absent.toml");
    let absent = absent.to_str().expect("a utf-8 path");

    let output = run(&["config", "defaults"], Some(absent));

    assert_eq!(parse(&stdout_of(&output)).len(), 22);
}
