//! The binary as a real `commit-msg` hook (spec §7 and §2, §8 criterion 13).
//!
//! Each test creates a throwaway repository under the system temp dir, writes
//! the hook script the README documents into that repository's
//! `.git/hooks/commit-msg`, and drives real `git commit` calls. Nothing here
//! reads or writes the developer's own `~/.gitconfig` or
//! `~/.config/commit-msg-attribution-checker/config.toml`: the child
//! environment points `HOME` and `XDG_CONFIG_HOME` into the scratch directory,
//! `GIT_CONFIG_GLOBAL` at a scratch file that holds only `user.name` and
//! `user.email`, sets `GIT_CONFIG_NOSYSTEM`, and removes the config override.
//!
//! The hook runs `verify` with no `--config`, so spec §3.2 falls through to the
//! built-in defaults: the scratch `XDG_CONFIG_HOME` holds no file. Identity
//! targets resolve from the scratch `GIT_CONFIG_GLOBAL`, which is why the
//! repository needs no `-c` flags on the commits it makes.

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

/// The command word spec §7's snippet carries: the binary by name, as a user
/// who put it on `PATH` would write it.
const SPEC_COMMAND: &str = "commit-msg-attribution-checker";

/// spec §7's install snippet, byte for byte, including the trailing newline.
const SPEC_SNIPPET: &str = "#!/bin/sh\nexec commit-msg-attribution-checker verify \"$1\"\n";

/// The message the hook must reject: criterion 1's trailer, one line of subject
/// so the trailer is line 3 (spec §5.3).
const ATTRIBUTED: &str = "Add the parser\n\nCo-authored-by: Claude <noreply@anthropic.com>\n";

/// The message the hook must accept: no marker for any shipped pattern.
const CLEAN: &str = "Add the parser\n\nBody\n";

/// The pattern a blocked commit's report has to name (spec §4.1).
const CLAUDE_PATTERN: &str = "co-authored-by.claude";

/// A scratch directory under the system temp dir: no extra crate, and no test
/// touches the real user profile.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            env::temp_dir().join(format!("cmac-hook-{name}-{}-{unique}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("creating the scratch directory");
        Self { path }
    }

    fn join(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// A throwaway repository with the documented hook installed.
struct Repo {
    scratch: Scratch,
    dir: PathBuf,
}

impl Repo {
    fn new(name: &str) -> Self {
        let scratch = Scratch::new(name);
        let dir = scratch.join("repo");
        fs::create_dir_all(&dir).expect("creating the repository directory");

        // The identity comes from here, so the assertion never depends on the
        // developer's `~/.gitconfig`: `GIT_CONFIG_GLOBAL` below points at this
        // file and `GIT_CONFIG_NOSYSTEM` drops the machine-wide one.
        fs::write(
            scratch.join("gitconfig"),
            "[user]\n\tname = Ada Lovelace\n\temail = ada@example.com\n",
        )
        .expect("writing the scratch git config");

        let repo = Self { scratch, dir };

        let init = repo.git(&["init", "--quiet", "."]);
        assert!(
            init.status.success(),
            "git init must succeed: {}",
            git_output(&init)
        );

        let hooks = repo
            .hook_path()
            .parent()
            .expect("the hooks directory")
            .to_owned();
        fs::create_dir_all(&hooks).expect("creating the hooks directory");
        fs::write(
            repo.hook_path(),
            hook_script(&shell_command(Path::new(BIN))),
        )
        .expect("writing the hook");
        make_executable(&repo.hook_path());

        repo
    }

    fn join(&self, relative: &str) -> PathBuf {
        self.scratch.join(relative)
    }

    fn hook_path(&self) -> PathBuf {
        self.dir.join(".git").join("hooks").join("commit-msg")
    }

    /// Run git inside the repository. The hook inherits this environment, which
    /// is why it sees the scratch `HOME` and `XDG_CONFIG_HOME` too.
    fn git(&self, args: &[&str]) -> Output {
        let mut command = Command::new("git");
        command.args(args).current_dir(&self.dir);
        isolate(&mut command, &self.scratch);
        command.output().expect("running git")
    }

    /// Run the binary under the same isolation the hook gets, for a verdict on a
    /// message the test wants to compare with what git recorded.
    fn verify(&self, message: &Path) -> Output {
        let mut command = Command::new(BIN);
        command.arg("verify").arg(message).current_dir(&self.dir);
        isolate(&mut command, &self.scratch);
        command.output().expect("running the binary")
    }

    /// Stage one file, so `git commit` has something to record and reaches the
    /// `commit-msg` hook.
    fn stage(&self, name: &str) {
        fs::write(self.dir.join(name), "content\n").expect("writing the staged file");
        let added = self.git(&["add", name]);
        assert!(added.status.success(), "git add: {}", git_output(&added));
    }

    fn message(&self, name: &str, bytes: &str) -> PathBuf {
        let path = self.join(&format!("msg-{name}.txt"));
        fs::write(&path, bytes).expect("writing the message file");
        path
    }

    /// `git commit -F <message>` plus extra flags, so no test needs an editor
    /// and the hook sees the fixture's bytes.
    fn commit(&self, message: &Path, extra: &[&str]) -> Output {
        let message = message.display().to_string();
        let mut args = vec!["commit", "--quiet", "-F", message.as_str()];
        args.extend_from_slice(extra);
        self.git(&args)
    }

    fn head(&self) -> Output {
        self.git(&["rev-parse", "HEAD"])
    }

    fn log(&self) -> Output {
        self.git(&["log", "--oneline"])
    }
}

/// The environment every child gets: git, the hook it runs, and `verify` all
/// see the same isolated world.
///
/// `HOME` and `XDG_CONFIG_HOME` point into the scratch directory, so the hook's
/// `verify` — which passes no `--config` — finds no file at the default path and
/// spec §3.2 step 4 applies the built-in defaults. `GIT_CONFIG_GLOBAL` is the
/// scratch identity file and `GIT_CONFIG_NOSYSTEM` drops the machine-wide one,
/// so `git var` resolves the same thing for every run and the developer's own
/// config is never read. The config override is removed for the same reason:
/// an inherited value would decide the outcome instead of the defaults.
fn isolate(command: &mut Command, scratch: &Scratch) {
    command
        .env("HOME", scratch.join("home"))
        .env("XDG_CONFIG_HOME", scratch.join("xdg"))
        .env("GIT_CONFIG_GLOBAL", scratch.join("gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove(ENV_CONFIG);
    // A repository or identity forced by the surrounding shell would move the
    // commit, or the value an identity target tests, out of the scratch repo.
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_AUTHOR_NAME",
        "GIT_AUTHOR_EMAIL",
        "GIT_COMMITTER_NAME",
        "GIT_COMMITTER_EMAIL",
    ] {
        command.env_remove(name);
    }
}

/// Mark the hook runnable where the platform has an executable bit. On Windows
/// git runs `sh` hooks regardless, so a no-op is correct there.
#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)
        .expect("the hook exists")
        .permissions()
        .mode();
    fs::set_permissions(path, fs::Permissions::from_mode(mode | 0o111))
        .expect("setting the hook executable");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

/// Both streams of a git run. Git shows a hook's output on its own stderr, but
/// the merge means the assertion holds whatever a given version does with the
/// two streams.
fn git_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Fail loudly, with the reason, when git cannot run a `sh` hook here.
///
/// A missing git is reported here rather than as an unexplained commit
/// failure. There is no `#[ignore]` and no skip: a silent pass would claim
/// criterion 13 without evidence. When the hook script itself cannot be started
/// the commit succeeds and the assertion in the test below fails with git's own
/// diagnostic in the message, which names the cause.
fn require_git() {
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => panic!(
            "criterion 13 needs git: `git --version` exited {:?}",
            output.status
        ),
        Err(err) => panic!("criterion 13 needs a git binary on PATH: {err}"),
    }
    // spec §7's shebang is /bin/sh, so a POSIX host must have it. Git for
    // Windows runs the same script through the `sh` that ships with git, which
    // this probe cannot name portably; the commit below is the evidence there.
    #[cfg(unix)]
    {
        let sh = Command::new("sh").arg("-c").arg("exit 0").output();
        assert!(
            sh.is_ok_and(|output| output.status.success()),
            "a POSIX host must provide sh for the hook script of spec §7"
        );
    }
}

// The escaping rule, pinned on a path that a real install can produce: a
// Windows drive with a space, and the characters that stay special inside
// double quotes. The assertion is the same on every host, so it is not a
// tautology of the platform the suite runs on.
#[test]
fn the_shell_command_quotes_and_forwards_slashes() {
    assert_eq!(
        shell_command(Path::new(r"C:\Program Files\cmac\bin\cmac.exe")),
        r#""C:/Program Files/cmac/bin/cmac.exe""#,
        "backslashes become separators and the space stays one word"
    );
    assert_eq!(
        shell_command(Path::new(r#"/opt/a $b `c" d/cmac"#)),
        r#""/opt/a \$b \`c\" d/cmac""#,
        "a character that expands or closes the quote is escaped"
    );
}

// spec §8 criterion 13, with spec §7's hook
#[test]
fn a_hook_blocks_an_attribution_commit_and_leaves_no_commit_object() {
    require_git();
    let repo = Repo::new("criterion-13");
    repo.stage("a.txt");
    let attributed = repo.message("attributed", ATTRIBUTED);

    let blocked = repo.commit(&attributed, &[]);
    let shown = git_output(&blocked);

    assert!(
        !blocked.status.success(),
        "the hook must abort the commit: {shown}"
    );
    // The block is the hook's doing and not an unrelated git failure: git shows
    // the hook's own report, ids and all.
    assert!(
        shown.contains("commit-msg-attribution-checker: attribution detected"),
        "the hook's report reaches the user: {shown}"
    );
    assert!(shown.contains(CLAUDE_PATTERN), "{shown}");
    assert!(shown.contains("1 match. Commit rejected."), "{shown}");

    // spec §1: the tool runs before git writes the object, so the rejected
    // attempt leaves an unborn branch. The exit statuses are the assertion; the
    // output text of a failed `git log` is not part of the contract.
    assert!(
        !repo.head().status.success(),
        "a rejected commit must leave no HEAD: {}",
        git_output(&repo.head())
    );
    assert!(
        !repo.log().status.success(),
        "a rejected commit must leave nothing to log: {}",
        git_output(&repo.log())
    );

    // The same hook lets a clean message through, so the rejection above is the
    // message's and not a hook that fails on everything.
    let clean = repo.commit(&repo.message("clean", CLEAN), &[]);
    assert!(
        clean.status.success(),
        "a clean message commits through the same hook: {}",
        git_output(&clean)
    );
    assert!(repo.head().status.success());

    // spec §2: `--no-verify` skips the hook. The same message and the same hook
    // now commit, which is the soft gate the spec admits to. The clean commit
    // consumed the staged file, so this one stages its own.
    repo.stage("b.txt");
    let bypassed = repo.commit(&attributed, &["--no-verify"]);
    assert!(
        bypassed.status.success(),
        "--no-verify skips the hook: {}",
        git_output(&bypassed)
    );
    assert!(
        !git_output(&bypassed).contains("attribution detected"),
        "the hook's report cannot appear when the hook did not run: {}",
        git_output(&bypassed)
    );

    // Two commits landed - the clean one and the bypassed one. The rejected
    // attempt is not a third, which is criterion 13's "no commit object".
    let count = repo.git(&["rev-list", "--count", "HEAD"]);
    assert_eq!(
        git_output(&count).trim(),
        "2",
        "the rejected attempt is not a commit: {}",
        git_output(&count)
    );
}

// The known gap of spec §2, asserted rather than assumed: the hook is installed
// and demonstrably live, and `--no-verify` still commits the message it rejects.
#[test]
fn the_hook_does_not_fire_for_git_commit_no_verify() {
    require_git();
    let repo = Repo::new("no-verify");
    repo.stage("a.txt");
    let attributed = repo.message("attributed", ATTRIBUTED);

    // Liveness first: without the flag the same hook and message are refused, so
    // a success below can only mean the hook was not run.
    let refused = repo.commit(&attributed, &[]);
    assert!(
        !refused.status.success(),
        "the fixture must be refused while the flag is absent: {}",
        git_output(&refused)
    );

    let bypassed = repo.commit(&attributed, &["--no-verify"]);
    assert!(
        bypassed.status.success(),
        "the flag bypasses the hook: {}",
        git_output(&bypassed)
    );

    // The gap is real, not a tolerance: `verify` rejects exactly the bytes git
    // recorded, so the repository now holds a commit the tool would refuse.
    let stored = repo.git(&["log", "-1", "--format=%B"]);
    assert!(
        stored.status.success(),
        "the bypassed commit is there: {}",
        git_output(&stored)
    );
    let stored_file = repo.join("stored-message.txt");
    fs::write(&stored_file, stored.stdout).expect("writing the recorded message");

    let verdict = repo.verify(&stored_file);
    assert_eq!(
        verdict.status.code(),
        Some(1),
        "the tool rejects the message that is now committed: {}",
        String::from_utf8_lossy(&verdict.stderr)
    );
    assert!(
        String::from_utf8_lossy(&verdict.stdout).contains(CLAUDE_PATTERN),
        "{:?}",
        String::from_utf8_lossy(&verdict.stdout)
    );
}

// The documented snippet and the installed one, reconciled: the README carries
// spec §7's text verbatim, the template reproduces it, and the installed script
// is that text with the command replaced by the quoted absolute binary path.
#[test]
fn the_readme_documents_the_installed_snippet() {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let readme = fs::read_to_string(&readme_path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", readme_path.display()));

    assert_eq!(
        hook_script(SPEC_COMMAND),
        SPEC_SNIPPET,
        "the template is spec §7's snippet, not a second copy of it"
    );
    assert!(
        readme.contains(&format!("```sh\n{SPEC_SNIPPET}```")),
        "README.md must carry spec §7's snippet verbatim in a `sh` block:\n{readme}"
    );

    let installed = hook_script(&shell_command(Path::new(BIN)));
    let expected = SPEC_SNIPPET.replace(SPEC_COMMAND, &shell_command(Path::new(BIN)));
    assert_eq!(
        installed, expected,
        "the installed script is spec §7's snippet with the command replaced"
    );

    // What the repository actually holds, so the claim covers the file on disk
    // and not only the value the helper returned.
    let repo = Repo::new("snippet");
    let on_disk = fs::read_to_string(repo.hook_path()).expect("the hook exists");
    assert_eq!(
        on_disk, expected,
        "the hook file in the repository is the documented snippet with the \
         binary resolved"
    );
    assert!(
        installed.ends_with(" verify \"$1\"\n"),
        "`$1` reaches the shell unexpanded: {installed:?}"
    );
}
