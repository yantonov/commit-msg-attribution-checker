//! The identity half of `verify` as the binary exposes it (spec §5.2, §8
//! criterion 12): the `--author`, `--committer` and `-v` flags are accepted,
//! the skip note reaches stderr, and a missing `git` binary is exit 2.
//!
//! The match output and the 0/1 exit statuses belong to `cmd-verify`, so every
//! test here asserts the identity resolution and nothing about the report.
//! Each child gets `--config` and a scratch `XDG_CONFIG_HOME`/`HOME`, so no run
//! reaches the machine's own config path.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_commit-msg-attribution-checker");

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
            "cmac-identity-bin-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("creating the scratch directory");
        Self { path }
    }

    fn join(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }

    /// The message file and a config the child reads, so neither is the
    /// machine's.
    fn message(&self) -> PathBuf {
        let path = self.join("msg.txt");
        fs::write(&path, "Subject\n").expect("writing the message file");
        let config = self.join("config.toml");
        fs::write(&config, "version = 1\n").expect("writing the config file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Run the binary with the scratch environment and, when given, a `PATH` that
/// replaces the inherited one.
fn run_in(dir: &Path, scratch: &Scratch, args: &[String], path: Option<&Path>) -> Output {
    let mut command = Command::new(BIN);
    command
        .args(args)
        .current_dir(dir)
        .env("XDG_CONFIG_HOME", &scratch.path)
        .env("HOME", &scratch.path);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.output().expect("running the binary")
}

/// `verify <message> --config <config>` plus any extra arguments, as owned
/// strings so a scratch path outlives the call.
fn args(scratch: &Scratch, message: &Path, extra: &[&str]) -> Vec<String> {
    let mut args = vec![
        "verify".to_owned(),
        message.display().to_string(),
        "--config".to_owned(),
        scratch.join("config.toml").display().to_string(),
    ];
    args.extend(extra.iter().map(|arg| (*arg).to_owned()));
    args
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is utf-8")
}

/// Fails loudly on a machine without git rather than letting a test pass for
/// the wrong reason.
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

#[test]
fn the_identity_flags_are_accepted() {
    let scratch = Scratch::new("flags");
    let message = scratch.message();

    let args = args(
        &scratch,
        &message,
        &[
            "--author",
            "Aider (aider)",
            "--committer",
            "Cursor <dev@cursor.com>",
            "-v",
        ],
    );

    let output = run_in(&scratch.path, &scratch, &args, None);

    let stderr = stderr(&output);
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("invalid"),
        "every declared flag parses: {stderr}"
    );
    assert!(
        stderr.contains("not implemented"),
        "the command still declines, so no caller reads a half-built check as clean: {stderr}"
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "stdout stays the report's alone");
}

#[test]
fn outside_a_repository_the_skip_is_reported_at_v() {
    // spec §5.2 and §8 criterion 12: the identity targets are skipped, and `-v`
    // names them. The exit status here is the not-implemented status, so only
    // the note is asserted; `cmd-verify` owns the exit-status half.
    require_git();
    let scratch = Scratch::new("skip");
    let message = scratch.message();
    let output = run_in(
        &scratch.path,
        &scratch,
        &args(&scratch, &message, &["-v"]),
        None,
    );

    let stderr = stderr(&output);
    assert!(stderr.contains("skipping author"), "{stderr}");
    assert!(stderr.contains("skipping committer"), "{stderr}");
    assert!(stderr.contains("not inside a repository"), "{stderr}");
}

#[test]
fn without_v_the_skip_is_not_reported() {
    // The note is a diagnostic the caller opts into, so the default run stays
    // quiet about a skip that changes nothing.
    require_git();
    let scratch = Scratch::new("quiet");
    let message = scratch.message();
    let output = run_in(
        &scratch.path,
        &scratch,
        &args(&scratch, &message, &[]),
        None,
    );

    let stderr = stderr(&output);
    assert!(!stderr.contains("skipping"), "{stderr}");
}

#[test]
fn inside_a_repository_with_a_committer_nothing_is_skipped() {
    // The resolved case, through the binary: `git var` answers, so `-v` has
    // nothing to report.
    require_git();
    let scratch = Scratch::new("in-repo");
    let message = scratch.message();
    for args in [
        ["init", "--quiet"].as_slice(),
        ["config", "user.name", "Ada Lovelace"].as_slice(),
        ["config", "user.email", "ada@example.com"].as_slice(),
    ] {
        let output = Command::new("git")
            .args(args)
            .current_dir(&scratch.path)
            .output()
            .expect("running git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = run_in(
        &scratch.path,
        &scratch,
        &args(&scratch, &message, &["-v"]),
        None,
    );

    let stderr = stderr(&output);
    assert!(!stderr.contains("skipping"), "{stderr}");
    assert!(stderr.contains("not implemented"), "{stderr}");
}

#[test]
fn a_missing_git_binary_is_exit_2_and_not_a_skip() {
    // spec §5.2: the tool shells out, so no binary is an I/O error. An empty
    // `PATH` is the only way to reach that from a test without uninstalling git;
    // the message file and config are given, so the failure is git's.
    let scratch = Scratch::new("missing-git");
    let message = scratch.message();
    let empty = scratch.join("empty-path");
    fs::create_dir_all(&empty).expect("creating the empty PATH directory");

    let output = run_in(
        &scratch.path,
        &scratch,
        &args(&scratch, &message, &["-v"]),
        Some(&empty),
    );

    let stderr = stderr(&output);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(
        !stderr.contains("skipping"),
        "a missing binary is an error, never a skip: {stderr}"
    );
    assert!(
        !stderr.contains("not implemented"),
        "the identity error is reported before the command declines: {stderr}"
    );
    assert!(stderr.contains("git"), "the message names git: {stderr}");
}
