//! `config create` contract (spec §6): write the built-in config to the
//! resolved path, create the parent directories, print the absolute path, and
//! never overwrite.
//!
//! Every test spawns the real binary, because what the command promises is a
//! file on disk, one line on stdout and an exit status. Each test names the
//! target with `--config` or the environment variable, so no run reads or
//! writes the machine's own config path.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_commit-msg-attribution-checker");

/// Environment variable naming a config file (spec §3.1).
const ENV_CONFIG: &str = "COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG";

/// A scratch directory under the system temp dir: no extra crate, and no test
/// touches the real user profile.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("cmac-create-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).expect("creating the scratch directory");
        Self { path }
    }

    /// A path below the scratch directory; the file itself is not created.
    fn target(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Run the binary with the config environment variable cleared unless the test
/// states it, so the machine's own override cannot decide the outcome.
fn run(args: &[&str], env_config: Option<&Path>) -> Output {
    run_in(args, env_config, None)
}

fn run_in(args: &[&str], env_config: Option<&Path>, cwd: Option<&Path>) -> Output {
    let mut command = Command::new(BIN);
    command.args(args);
    if let Some(value) = env_config {
        command.env(ENV_CONFIG, value);
    } else {
        command.env_remove(ENV_CONFIG);
    }
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    command.output().expect("running the binary")
}

/// The absolute path the command printed, asserting exit 0 and a silent stderr.
fn created_path(output: &Output) -> String {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "a successful create says nothing on stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout is utf-8");
    assert_eq!(
        stdout.lines().count(),
        1,
        "exactly one line, so the output is usable as a file name: {stdout:?}"
    );
    stdout
        .strip_suffix('\n')
        .expect("a trailing newline")
        .to_string()
}

/// The common assertion set for the refuse path: exit 2, the path on stderr,
/// nothing on stdout.
fn refused(output: &Output, path: &Path) {
    assert_eq!(
        output.status.code(),
        Some(2),
        "an existing target is a usage error (spec §6)"
    );
    assert!(
        output.stdout.is_empty(),
        "the path goes to stderr here, so a caller reading stdout sees no path: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr is utf-8");
    assert!(
        stderr.contains(&path.display().to_string()),
        "stderr names the path {}: {stderr}",
        path.display()
    );
}

fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).expect("a working directory exists")
}

#[test]
fn a_fresh_path_gets_the_file_and_the_absolute_path_on_stdout() {
    let scratch = Scratch::new();
    let target = scratch.target("config.toml");
    let expected = absolute(&target);

    let output = run(
        &["--config", target.to_str().unwrap(), "config", "create"],
        None,
    );
    let printed = created_path(&output);

    assert_eq!(
        Path::new(&printed),
        expected,
        "the printed path is the file it wrote"
    );
    assert!(
        Path::new(&printed).is_absolute(),
        "spec §6 prints an absolute path"
    );
    assert_eq!(
        fs::read_to_string(&target).expect("the file exists"),
        stdout_of(&run(&["config", "defaults"], None)),
        "spec §10: the written file and the printed defaults share one serializer"
    );
}

#[test]
fn the_written_file_loads_through_the_config_model() {
    // Done when: the created file is a config the binary can apply. Parsing it
    // here with the same TOML types the binary uses is the check available
    // before `verify` lands.
    let scratch = Scratch::new();
    let target = scratch.target("config.toml");
    created_path(&run(
        &["--config", target.to_str().unwrap(), "config", "create"],
        None,
    ));

    let text = fs::read_to_string(&target).expect("the file exists");
    let document: toml::Value = toml::from_str(&text).expect("the file is TOML");

    assert_eq!(
        document.get("version").and_then(toml::Value::as_integer),
        Some(1),
        "spec §3.4: the file leads with version = 1"
    );
    let patterns = document
        .get("pattern")
        .and_then(toml::Value::as_array)
        .expect("an array of [[pattern]] tables");
    assert_eq!(patterns.len(), 22, "spec §4: the full default set");
    for pattern in patterns {
        let id = pattern
            .get("id")
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("a pattern without an id: {pattern}"));
        let regex = pattern
            .get("regex")
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("{id}: no regex"));
        // A pattern that does not compile would make `Config::parse` reject the
        // file, so every entry is compiled here through the same crate.
        regex::Regex::new(regex).unwrap_or_else(|err| panic!("{id}: {err}"));
        assert!(
            !pattern
                .get("description")
                .and_then(toml::Value::as_str)
                .unwrap_or("")
                .trim()
                .is_empty(),
            "{id}: a pattern without a description names no reason"
        );
    }
}

#[test]
fn a_second_run_exits_2_prints_the_path_and_leaves_the_file_identical() {
    // spec §8 criterion 10, and the hard constraint in AGENTS.md.
    let scratch = Scratch::new();
    let target = scratch.target("config.toml");
    let args = ["--config", target.to_str().unwrap(), "config", "create"];
    let first = created_path(&run(&args, None));
    let before = fs::read(&target).expect("the first run wrote the file");

    let second = run(&args, None);
    refused(&second, &target);

    let after = fs::read(&target).expect("the refuse path leaves the file");
    assert_eq!(before, after, "the file is byte-identical after a refusal");
    // The user's file must still be the complete default set, not a partial
    // write from the second run.
    assert_eq!(
        after,
        stdout_of(&run(&["config", "defaults"], None)).into_bytes()
    );
    assert_eq!(
        fs::read_dir(&scratch.path).unwrap().count(),
        1,
        "no temporary file is left beside the config"
    );
    assert!(Path::new(&first).is_file());
}

#[test]
fn a_second_run_leaves_a_hand_edited_file_byte_identical() {
    // Criterion 10 with content the tool did not write: an overwrite here would
    // destroy a user's edit, which spec §6 forbids.
    let scratch = Scratch::new();
    let target = scratch.target("config.toml");
    let edited =
        "[[pattern]]\nid = \"mine\"\ndescription = \"my rule\"\ntarget = \"line\"\nregex = \"x\"\n";
    fs::write(&target, edited).expect("writing the user config");

    let output = run(
        &["--config", target.to_str().unwrap(), "config", "create"],
        None,
    );
    refused(&output, &target);

    assert_eq!(fs::read_to_string(&target).unwrap(), edited);
}

#[test]
fn parent_directories_that_do_not_exist_are_created() {
    let scratch = Scratch::new();
    // Two levels that the scratch directory does not have yet.
    let target = scratch.target("one/two/config.toml");
    assert!(!scratch.path.join("one").exists());

    let printed = created_path(&run(
        &["--config", target.to_str().unwrap(), "config", "create"],
        None,
    ));

    assert_eq!(Path::new(&printed), absolute(&target));
    assert!(target.is_file(), "{}: no file", target.display());
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        stdout_of(&run(&["config", "defaults"], None))
    );
}

#[test]
fn a_relative_config_path_prints_an_absolute_path() {
    // The path is resolved against the working directory of the process, which
    // this test sets, so the assertion does not depend on the test runner's.
    let scratch = Scratch::new();
    let relative = Path::new("nested/config.toml");
    let expected = absolute(&scratch.path.join(relative));

    let output = run_in(
        &["--config", "nested/config.toml", "config", "create"],
        None,
        Some(&scratch.path),
    );
    let printed = created_path(&output);

    assert!(
        Path::new(&printed).is_absolute(),
        "a relative --config must still print an absolute path: {printed}"
    );
    assert_eq!(Path::new(&printed), expected);
    assert!(scratch.path.join(relative).is_file());
}

#[test]
fn the_environment_variable_supplies_the_target() {
    let scratch = Scratch::new();
    let target = scratch.target("from-env/config.toml");

    let output = run(&["config", "create"], Some(&target));
    let printed = created_path(&output);

    assert_eq!(Path::new(&printed), absolute(&target));
    assert!(target.is_file());
}

#[test]
fn the_flag_wins_over_the_environment_variable() {
    // spec §3.2 step 1 before step 2, so an explicit flag cannot be redirected
    // by the environment.
    let scratch = Scratch::new();
    let flag = scratch.target("from-flag.toml");
    let env_target = scratch.target("from-env.toml");

    let printed = created_path(&run(
        &["--config", flag.to_str().unwrap(), "config", "create"],
        Some(&env_target),
    ));

    assert_eq!(Path::new(&printed), absolute(&flag));
    assert!(flag.is_file());
    assert!(
        !env_target.exists(),
        "the environment variable must not be used when --config is given"
    );
}

#[test]
fn a_created_file_matches_config_defaults_byte_for_byte() {
    // The cross-command identity deferred from `cmd-config-defaults`: the two
    // commands share one serializer, so their bytes cannot diverge.
    let scratch = Scratch::new();
    let target = scratch.target("config.toml");
    created_path(&run(
        &["--config", target.to_str().unwrap(), "config", "create"],
        None,
    ));

    let written = fs::read(&target).expect("the file exists");
    let printed = run(&["config", "defaults"], None);
    assert_eq!(
        printed.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&printed.stderr)
    );

    assert_eq!(
        written, printed.stdout,
        "the file and stdout are the same bytes"
    );
    assert!(
        !written.is_empty(),
        "an empty file would compare equal to nothing"
    );
}

#[test]
fn the_created_file_is_not_empty_and_ends_with_a_newline() {
    // A half-written file is the failure mode the temporary file rules out; a
    // truncated one would also be smaller than the printed document.
    let scratch = Scratch::new();
    let target = scratch.target("config.toml");
    created_path(&run(
        &["--config", target.to_str().unwrap(), "config", "create"],
        None,
    ));

    let written = fs::read(&target).unwrap();
    assert_eq!(
        written.last().copied(),
        Some(b'\n'),
        "the document ends with a newline, so an editor shows no missing line"
    );
    assert!(written.len() > 1024, "{} bytes is too few", written.len());
}

fn stdout_of(output: &Output) -> String {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}
