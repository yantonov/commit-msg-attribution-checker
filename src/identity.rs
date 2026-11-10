//! Identity resolution (spec §5.2): the values the `author` and `committer`
//! targets test.
//!
//! `git var` is the authority, not `GIT_AUTHOR_*` and `GIT_COMMITTER_*`. Git
//! leaves the committer variables unset when the committer defaults to the
//! author, so reading the environment would disagree with the ident git is
//! about to record.
//!
//! The decision is split from the I/O. [`decide`] takes what one git run
//! observed and returns the value or the error, so "outside a repository is a
//! skip", "a failed `git var` inside one is an error" and "the flag is used as
//! given" are unit tests. [`resolve`] is the thin wrapper that runs git.

// Reachable once `verify` lands; clippy runs with -D warnings, so the module
// stays quiet until then.
#![allow(dead_code)]

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Output};

/// The program every question goes through.
const GIT: &str = "git";

/// The `git var` names spec §5.2 names.
pub const AUTHOR_IDENT: &str = "GIT_AUTHOR_IDENT";
pub const COMMITTER_IDENT: &str = "GIT_COMMITTER_IDENT";

/// One run's identity inputs.
///
/// Every outside fact is injected: the two flags, the program, and the
/// directory git runs in. A test therefore needs no process-wide state, no
/// particular working directory, and no environment variable.
#[derive(Debug, Clone)]
pub struct Inputs<'a> {
    /// `--author <ident>`, used verbatim (spec §5.2).
    pub author: Option<&'a str>,
    /// `--committer <ident>`, same rules.
    pub committer: Option<&'a str>,
    /// The program to run. A path rather than a name so a test can point at one
    /// that does not exist.
    pub git: PathBuf,
    /// The directory git runs in. `None` keeps the caller's own.
    pub dir: Option<PathBuf>,
}

impl<'a> Inputs<'a> {
    /// The production inputs: the flags the user passed, and `git` from `PATH`
    /// in the process's own directory.
    pub fn new(author: Option<&'a str>, committer: Option<&'a str>) -> Self {
        Self {
            author,
            committer,
            git: PathBuf::from(GIT),
            dir: None,
        }
    }
}

/// What one identity target tests, or why it tests nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// The text a pattern is tested against.
    Resolved(String),
    /// No value: git cannot name a repository here, so it will not record an
    /// identity from this directory (spec §5.2). This is the one skip; a
    /// missing git binary is an error.
    OutsideRepository,
}

impl Value {
    /// The text a pattern tests, or `None` when the target is skipped.
    pub fn resolved(&self) -> Option<&str> {
        match self {
            Value::Resolved(text) => Some(text),
            Value::OutsideRepository => None,
        }
    }
}

/// Both identity targets as one run resolved them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub author: Value,
    pub committer: Value,
}

/// What one target's git work observed, so [`decide`] needs no process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observed {
    /// No git call was needed: the flag supplied the value.
    Flag { value: String },
    /// `git rev-parse --git-dir` named no repository, which is the skip case.
    OutsideRepository,
    /// `git var <variable>` exited 0 with this stdout.
    Ident {
        variable: &'static str,
        stdout: String,
    },
    /// `git var <variable>` ran inside a repository and failed.
    IdentFailed {
        variable: &'static str,
        stderr: String,
    },
}

/// Decide one target from what was observed.
///
/// Pure on purpose: [`resolve`] produces [`Observed`], so every branch that
/// decides between a value, a skip and an error is testable without a
/// repository, without a git binary, and without changing the environment.
pub fn decide(observed: &Observed) -> Result<Value, IdentityError> {
    match observed {
        // spec §5.2: the flag value is used as given. Deliberately not passed
        // through `render` — the caller may test a pattern against text git
        // would never print, and this is the one input that needs no
        // repository.
        Observed::Flag { value } => Ok(Value::Resolved(value.clone())),
        Observed::OutsideRepository => Ok(Value::OutsideRepository),
        Observed::Ident { variable, stdout } => render(variable, stdout).map(Value::Resolved),
        Observed::IdentFailed { variable, stderr } => Err(IdentityError::Var {
            variable,
            stderr: stderr.clone(),
        }),
    }
}

/// Render one `git var` ident as spec §5.2 words it: `Name <email>`, one space
/// before the angle bracket.
///
/// `git var GIT_AUTHOR_IDENT` prints `Name <email> 1794295443 +0100`. The
/// timestamp and the timezone are dropped, because a pattern tests who, not
/// when, and the name is trimmed so exactly one space precedes the bracket.
fn render(variable: &'static str, stdout: &str) -> Result<String, IdentityError> {
    let text = stdout.trim();
    // The last `<` rather than the first, so a name that itself contains an
    // angle bracket still splits at the email.
    let unparsable = || IdentityError::Unparsable {
        variable,
        output: text.to_owned(),
    };
    let open = text.rfind('<').ok_or_else(unparsable)?;
    let close = text[open..]
        .find('>')
        .map(|offset| open + offset)
        .ok_or_else(unparsable)?;

    let name = text[..open].trim();
    let email = &text[open..=close];
    Ok(format!("{name} {email}"))
}

/// Resolve both identity targets for one run.
///
/// The repository probe runs once, and only when a target still needs git: a
/// pair of flags is a complete answer, which is what lets a caller test a
/// pattern outside a repository (spec §5.2).
pub fn resolve(inputs: &Inputs<'_>) -> Result<Identity, IdentityError> {
    let repository = if inputs.author.is_some() && inputs.committer.is_some() {
        None
    } else {
        Some(in_repository(inputs)?)
    };

    Ok(Identity {
        author: decide(&observe(inputs, inputs.author, AUTHOR_IDENT, repository)?)?,
        committer: decide(&observe(
            inputs,
            inputs.committer,
            COMMITTER_IDENT,
            repository,
        )?)?,
    })
}

/// Whether git can name a repository from the injected directory.
///
/// `rev-parse --git-dir` answers for a work tree and for a bare repository
/// alike, and fails outside both, so its status is the skip signal (spec §5.2).
/// A `git var` failure is deliberately not the signal: outside a repository
/// `git var` still prints an identity built from the user's global config, and
/// git will not record that as this commit's committer.
fn in_repository(inputs: &Inputs<'_>) -> Result<bool, IdentityError> {
    Ok(git(inputs, &["rev-parse", "--git-dir"])?.status.success())
}

/// Observe one target: the flag when there is one, otherwise the probe's
/// answer, otherwise `git var`.
fn observe(
    inputs: &Inputs<'_>,
    flag: Option<&str>,
    variable: &'static str,
    in_repository: Option<bool>,
) -> Result<Observed, IdentityError> {
    if let Some(value) = flag {
        return Ok(Observed::Flag {
            value: value.to_owned(),
        });
    }
    // A target without a flag asks git, so the probe answered for it.
    if in_repository == Some(false) {
        return Ok(Observed::OutsideRepository);
    }

    let output = git(inputs, &["var", variable])?;
    if output.status.success() {
        return Ok(Observed::Ident {
            variable,
            // Lossy rather than an error: a non-UTF-8 ident cannot match a
            // pattern, and the replacement characters make that visible in the
            // report instead of hiding the value.
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        });
    }

    Ok(Observed::IdentFailed {
        variable,
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

/// Run one git command, in the injected directory.
fn git(inputs: &Inputs<'_>, args: &[&str]) -> Result<Output, IdentityError> {
    let mut command = Command::new(&inputs.git);
    if let Some(dir) = &inputs.dir {
        command.current_dir(dir);
    }
    // Nothing suppresses a pager or a credential prompt here: `rev-parse` and
    // `var` neither page nor talk to a remote. `output` also closes stdin, so
    // no run can wait for input a hook does not have.
    //
    // A failed spawn means no question could be asked at all, which includes
    // the missing binary. spec §5.2 makes that exit 2, never a skip.
    command
        .args(args)
        .output()
        .map_err(|source| IdentityError::Git {
            program: inputs.git.clone(),
            source,
        })
}

/// An identity that cannot be resolved. Every variant is an I/O error, which
/// spec §6 maps to exit status 2.
#[derive(Debug)]
pub enum IdentityError {
    /// The program could not be started: absent, or not executable.
    Git { program: PathBuf, source: io::Error },
    /// `git var` failed inside a repository, so no value can be tested.
    Var {
        variable: &'static str,
        stderr: String,
    },
    /// `git var` printed something that is not `Name <email>`.
    Unparsable {
        variable: &'static str,
        output: String,
    },
}

impl IdentityError {
    /// spec §6: an I/O error exits 2, and the hook and CI branch on that.
    pub fn exit_code(&self) -> u8 {
        2
    }
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityError::Git { program, source } => {
                write!(f, "{}: {source}", program.display())
            }
            IdentityError::Var { variable, stderr } => {
                write!(f, "git var {variable}: {stderr}")
            }
            IdentityError::Unparsable { variable, output } => {
                write!(f, "git var {variable}: not an identity: {output}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn resolved(value: Value) -> String {
        match value {
            Value::Resolved(text) => text,
            Value::OutsideRepository => panic!("expected a value, got a skip"),
        }
    }

    #[test]
    fn a_flag_value_is_used_as_given() {
        // The asymmetry with `Ident`: no rendering, no trimming, no timestamp
        // stripping, so a caller can aim a pattern at text git would not print.
        let observed = Observed::Flag {
            value: "  Aider (aider) <aider@example.com> tail ".to_owned(),
        };

        let value = decide(&observed).expect("a flag needs no git");

        assert_eq!(resolved(value), "  Aider (aider) <aider@example.com> tail ");
    }

    #[test]
    fn an_ident_renders_the_name_and_email_and_drops_the_timestamp() {
        // The shape `git var GIT_AUTHOR_IDENT` prints: name, bracket, epoch
        // seconds, timezone offset.
        let observed = Observed::Ident {
            variable: AUTHOR_IDENT,
            stdout: "Ada   Lovelace <ada@example.com> 1794295443 +0100\n".to_owned(),
        };

        let value = decide(&observed).expect("a real ident renders");

        assert_eq!(
            resolved(value),
            "Ada   Lovelace <ada@example.com>",
            "spec §5.2: `Name <email>`, trimmed, timestamp dropped"
        );
    }

    #[test]
    fn rendering_puts_exactly_one_space_before_the_bracket() {
        let observed = Observed::Ident {
            variable: COMMITTER_IDENT,
            stdout: "  Aider (aider)   <aider@example.com>   1794295443 -0500  ".to_owned(),
        };

        let value = decide(&observed).expect("a real ident renders");

        assert_eq!(resolved(value), "Aider (aider) <aider@example.com>");
    }

    #[test]
    fn outside_a_repository_is_a_skip() {
        let value = decide(&Observed::OutsideRepository).expect("a skip is not an error");

        assert_eq!(value, Value::OutsideRepository);
        assert_eq!(value.resolved(), None, "no pattern is tested");
    }

    #[test]
    fn a_failed_git_var_is_an_error_not_a_skip() {
        // Inside a repository `git var` succeeding is the normal case, so a
        // failure there cannot be read as "no repository" (spec §5.2).
        let error = decide(&Observed::IdentFailed {
            variable: AUTHOR_IDENT,
            stderr: "fatal: broken repository".to_owned(),
        })
        .expect_err("a failed git var is an error");

        assert_eq!(error.exit_code(), 2, "{error}");
        assert!(error.to_string().contains("broken repository"), "{error}");
        assert!(error.to_string().contains(AUTHOR_IDENT), "{error}");
    }

    #[test]
    fn an_ident_that_is_not_name_and_email_is_an_error() {
        // No bracket pair means no email to test, and guessing a rendering
        // would test text git never printed. spec §6: exit 2.
        for output in ["", "Ada Lovelace", "Ada <ada@example.com"] {
            let error = decide(&Observed::Ident {
                variable: AUTHOR_IDENT,
                stdout: output.to_owned(),
            })
            .expect_err("an ident without an email pair is unparsable");

            assert_eq!(error.exit_code(), 2, "{output:?}: {error}");
            assert!(matches!(error, IdentityError::Unparsable { .. }), "{error}");
        }
    }
    #[test]
    fn a_name_that_holds_an_angle_bracket_splits_at_the_email() {
        // Names are free text, so the last bracket is the email's.
        let value = decide(&Observed::Ident {
            variable: AUTHOR_IDENT,
            stdout: "A <B> Person <ab@example.com> 1794295443 +0000".to_owned(),
        })
        .expect("a bracketed name still renders");

        assert_eq!(resolved(value), "A <B> Person <ab@example.com>");
    }

    /// Creates a scratch directory under the system temp dir.
    fn scratch(name: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cmac-identity-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("creating the scratch directory");
        dir
    }

    /// Fails loudly on a machine without git, rather than letting the tests
    /// below pass for the wrong reason.
    fn require_git() {
        match Command::new("git").arg("--version").output() {
            Ok(output) if output.status.success() => {}
            Ok(output) => panic!(
                "these tests need a working git: `git --version` exited {:?}",
                output.status
            ),
            Err(err) => panic!("these tests need a git binary on PATH: {err}"),
        }
    }

    /// A repository with a repo-local name and email, so the identity does not
    /// depend on the machine's global config.
    fn repository(name: &str, user_name: &str, user_email: &str) -> PathBuf {
        require_git();
        let dir = scratch(name);
        run_git(&dir, &["init", "--quiet"]);
        run_git(&dir, &["config", "user.name", user_name]);
        run_git(&dir, &["config", "user.email", user_email]);
        dir
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap_or_else(|err| panic!("running git {args:?}: {err}"));
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn inputs(git: &Path, dir: Option<&Path>) -> Inputs<'static> {
        Inputs {
            author: None,
            committer: None,
            git: git.to_path_buf(),
            dir: dir.map(Path::to_path_buf),
        }
    }

    #[test]
    fn the_git_var_value_is_used_inside_a_repository() {
        // spec §5.2: `git var` is the authority, and the rendered value is what
        // a pattern tests.
        let dir = repository("in-repo", "Ada Lovelace", "ada@example.com");

        let identity = resolve(&inputs(Path::new("git"), Some(&dir)))
            .unwrap_or_else(|err| panic!("resolving inside a repository: {err}"));

        assert_eq!(
            identity.author,
            Value::Resolved("Ada Lovelace <ada@example.com>".to_owned())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_flag_wins_over_the_resolved_value() {
        let dir = repository("flag-wins", "Ada Lovelace", "ada@example.com");
        let mut inputs = inputs(Path::new("git"), Some(&dir));
        inputs.author = Some("Aider (aider) <aider@example.com>");

        let identity = resolve(&inputs).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            identity.author,
            Value::Resolved("Aider (aider) <aider@example.com>".to_owned()),
            "spec §5.2: the flag overrides the resolved value"
        );
        // The committer keeps git's answer, so the override is per target.
        assert_eq!(
            identity.committer,
            Value::Resolved("Ada Lovelace <ada@example.com>".to_owned())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn both_flags_need_no_repository_and_start_no_git() {
        // The program is pointed at a name no repository and no installation can
        // answer, so a successful resolution here proves the flags short-circuit
        // every git call rather than merely working inside a repository.
        let mut inputs = inputs(
            Path::new("cmac-identity-no-such-git-program"),
            Some(Path::new("/")),
        );
        inputs.author = Some("A (aider) <a@example.com>");
        inputs.committer = Some("B <b@cursor.com>");

        let identity = resolve(&inputs).expect("flags need no git");

        assert_eq!(
            identity.author,
            Value::Resolved("A (aider) <a@example.com>".to_owned())
        );
        assert_eq!(
            identity.committer,
            Value::Resolved("B <b@cursor.com>".to_owned())
        );
    }

    #[test]
    fn the_committer_falls_back_to_the_author() {
        // git derives the committer ident from the same user.name and user.email
        // when nothing sets a separate committer, and leaves GIT_COMMITTER_* out
        // of the environment entirely. Reading the environment would disagree
        // with the ident git records; `git var` cannot.
        let dir = repository("committer-defaults", "Ada Lovelace", "ada@example.com");

        let identity =
            resolve(&inputs(Path::new("git"), Some(&dir))).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(identity.committer, identity.author);
        assert_eq!(
            identity.committer,
            Value::Resolved("Ada Lovelace <ada@example.com>".to_owned())
        );

        // Tied to git's own answer, so the equality above is not an artifact of
        // our rendering: no `committer.*` config exists, and git still prints a
        // committer ident.
        let raw = Command::new("git")
            .args(["var", COMMITTER_IDENT])
            .current_dir(&dir)
            .output()
            .expect("running git var");
        assert!(
            String::from_utf8_lossy(&raw.stdout).contains("<ada@example.com>"),
            "git prints the author's email as the committer: {}",
            String::from_utf8_lossy(&raw.stdout)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_separate_committer_is_resolved_from_its_own_git_var() {
        let dir = repository("separate-committer", "Ada Lovelace", "ada@example.com");
        run_git(&dir, &["config", "committer.name", "Bob Committer"]);
        run_git(&dir, &["config", "committer.email", "bob@example.com"]);

        let identity =
            resolve(&inputs(Path::new("git"), Some(&dir))).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            identity.author,
            Value::Resolved("Ada Lovelace <ada@example.com>".to_owned())
        );
        assert_eq!(
            identity.committer,
            Value::Resolved("Bob Committer <bob@example.com>".to_owned())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn outside_a_repository_the_targets_are_skipped() {
        // spec §5.2 and §8 criterion 12 at the resolution level: a directory no
        // repository claims is a skip, and the run still succeeds.
        require_git();
        let dir = scratch("outside");
        let probe = Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(&dir)
            .output()
            .expect("running the probe");
        assert!(
            !probe.status.success(),
            "the fixture must be outside a repository; {} is inside one, so \
             point TMPDIR somewhere git does not claim",
            dir.display()
        );

        let identity = resolve(&inputs(Path::new("git"), Some(&dir)))
            .unwrap_or_else(|err| panic!("a skip is not an error: {err}"));

        assert_eq!(identity.author, Value::OutsideRepository);
        assert_eq!(identity.committer, Value::OutsideRepository);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn git_var_succeeds_outside_a_repository_so_its_failure_is_not_the_skip_signal() {
        // The probe exists because of this: outside a repository `git var` still
        // prints an ident, built from the user's global config, and exits 0.
        // Using its exit status as the skip signal would report an identity that
        // git will not record here; using a missing binary's failure as one would
        // hide a real error. `rev-parse --git-dir` is the honest question.
        require_git();
        let dir = scratch("git-var-outside");
        let output = Command::new("git")
            .args(["var", AUTHOR_IDENT])
            .current_dir(&dir)
            .output()
            .expect("running git var");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let _ = fs::remove_dir_all(&dir);

        assert!(
            output.status.success(),
            "git var needs no repository, so this is the trap the probe avoids: {stdout}"
        );
        assert!(
            !stdout.trim().is_empty(),
            "and it prints an ident: {stdout}"
        );
    }

    #[test]
    fn a_missing_git_binary_is_an_error_not_a_skip() {
        // spec §5.2: the tool shells out, so no binary is exit 2. The path is
        // injected rather than the binary uninstalled.
        let missing = std::env::temp_dir().join("cmac-identity-no-such-git-program");

        let error = resolve(&inputs(&missing, None))
            .expect_err("a missing git binary is an error, not a skip");

        assert_eq!(error.exit_code(), 2, "{error}");
        assert!(matches!(error, IdentityError::Git { .. }), "{error}");
        assert!(
            error
                .to_string()
                .contains("cmac-identity-no-such-git-program"),
            "the message names the program it could not run: {error}"
        );
    }

    #[test]
    fn a_missing_git_binary_fails_even_when_a_repository_would_be_skipped() {
        // The opposite order of the case above: the probe is the first git
        // question, so the error must come from the probe and not be read as
        // "outside a repository".
        let missing = std::env::temp_dir().join("cmac-identity-no-such-git-probe");
        let mut inputs = inputs(&missing, None);
        inputs.committer = Some("B <b@example.com>");

        let error = resolve(&inputs).expect_err("the probe cannot run");

        assert_eq!(error.exit_code(), 2, "{error}");
        assert!(matches!(error, IdentityError::Git { .. }), "{error}");
    }
}
