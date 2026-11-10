//! Command implementations (spec §6).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::defaults::{default_patterns, defaults_toml};
use crate::engine;
use crate::identity::{self, Identity, Value};
use crate::message::Message;
use crate::report;
use crate::resolve::{self, Inputs};

/// Prefix on every diagnostic, so a hook log names the tool (spec §6).
const NAME: &str = "commit-msg-attribution-checker";

/// `verify <msg-file>`: check one message file (spec §5.4, §6).
///
/// Exit status is 0 for a clean message, 1 for a match and 2 for an error;
/// nothing else leaves this function. `--json` changes the format of the report
/// and nothing else: the same message yields the same status either way, which
/// is what spec §5.4's "Exit codes do not change" means.
///
/// The three inputs are read in the order the user can fix them: the message
/// file, then the config, then the identity. A bad message path is the most
/// common user error and a file read is cheap, so checking it before the config
/// means a mistyped path is reported as a mistyped path even when the config is
/// also broken. The message is then read exactly once: [`Message`] owns the
/// parsed file and the identity resolution touches no file, so no second read
/// can disagree with the first.
pub fn verify(
    msg_file: &Path,
    config_flag: Option<PathBuf>,
    inputs: &identity::Inputs<'_>,
    verbose: bool,
    json: bool,
) -> ExitCode {
    // spec §6: an unreadable message is an I/O error, and the message names the
    // path because that is what the user has to correct.
    let message = match Message::read(msg_file) {
        Ok(message) => message,
        Err(error) => {
            eprintln!("{NAME}: verify: {error}");
            return ExitCode::from(error.exit_code());
        }
    };

    let resolved = match resolve::resolve(&Inputs::from_env(config_flag), default_patterns) {
        Ok(resolved) => resolved,
        Err(error) => {
            eprintln!("{NAME}: verify: {error}");
            return ExitCode::from(error.exit_code());
        }
    };

    let identity = match identity::resolve(inputs) {
        Ok(identity) => identity,
        Err(error) => {
            eprintln!("{NAME}: verify: {error}");
            return ExitCode::from(error.exit_code());
        }
    };

    // spec §5.2: the skip is a note, not silence and not a failure. stderr is
    // where every diagnostic goes, so `-v` leaves stdout byte-identical.
    if verbose {
        for note in skip_notes(&identity) {
            eprintln!("{note}");
        }
    }

    // spec §5.4: a clean message writes nothing at all on the human path, which
    // is what the renderer's `None` says. The JSON path writes `{"matches":[]}`
    // instead, because a CI consumer must tell an empty result from a silent
    // failure. Every error above already returned before this point, so neither
    // format can put a partial document on stdout.
    let matches = engine::run(&resolved.config, &message, Some(&identity));
    let report = if json {
        report::render_json(&matches)
    } else {
        let Some(block) = report::render(&matches) else {
            return ExitCode::SUCCESS;
        };
        block
    };
    // spec §6: 1 means attribution was found. It is read off the matches rather
    // than off the format, which is the whole point of the flag not changing the
    // status: an empty JSON document exits 0 like the silent human path.
    let status = if matches.is_empty() { 0 } else { 1 };

    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(report.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => ExitCode::from(status),
        Err(err) => {
            // The report did not reach the caller, so it must not be read as a
            // rejection the caller can act on: the write failure is an I/O
            // error of spec §6, and the hook keeps stderr instead of a block.
            eprintln!("{NAME}: verify: writing to stdout: {err}");
            ExitCode::from(2)
        }
    }
}

/// The `-v` notes for each identity target that contributed no value.
///
/// Pure, so the criterion 12 decision — a skip prints a note and changes no
/// outcome — is testable without a binary or a repository. stderr is where the
/// caller writes them, like every other diagnostic (spec §6), so `verify`'s
/// stdout stays the report and only the report.
fn skip_notes(identity: &Identity) -> Vec<String> {
    let mut notes = Vec::new();
    for (target, value) in [
        ("author", &identity.author),
        ("committer", &identity.committer),
    ] {
        if *value == Value::OutsideRepository {
            notes.push(format!(
                "{NAME}: verify: skipping {target}: not inside a repository"
            ));
        }
    }
    notes
}

/// `config defaults`: write the built-in config as TOML to stdout.
///
/// The bytes come from [`defaults_toml`] untouched, because spec §10 makes that
/// one serializer the source of both this output and the file `config create`
/// writes; formatting anything here would let the two diverge.
///
/// The user config is deliberately not resolved (spec §6): this command answers
/// what the binary ships, so a missing or broken `--config` path must not
/// change it.
pub fn config_defaults() -> ExitCode {
    let mut stdout = io::stdout().lock();
    match write_defaults(&mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        // `write_all` rather than `print!` so a broken pipe or a full disk
        // arrives here as an error instead of a panic; spec §6 makes it exit 2.
        Err(err) => {
            eprintln!("commit-msg-attribution-checker: config defaults: writing to stdout: {err}");
            ExitCode::from(2)
        }
    }
}

fn write_defaults(out: &mut impl Write) -> io::Result<()> {
    out.write_all(defaults_toml().as_bytes())?;
    // An explicit flush reports a failure on the last buffered block, which a
    // process exit would otherwise swallow.
    out.flush()
}

/// `config show`: write the effective config to stdout, with a header comment
/// naming the source (spec §6).
///
/// The body is the loaded [`Config`](crate::config::Config) re-serialized, not
/// the file text: a hand-edited file keeps its unreadable formatting in the
/// file, and the command exists to show what `verify` will apply.
///
/// Every load failure is exit 2 with the reason on stderr (spec §3.2, §6). The
/// message is printed once: [`LoadError`](crate::resolve::LoadError) already
/// names the path.
pub fn config_show(flag: Option<PathBuf>) -> ExitCode {
    let inputs = Inputs::from_env(flag);
    // `default_patterns` is the resolver's step 4, so the fallback list shown
    // here is the shipped one (spec §3.2).
    let resolved = match resolve::resolve(&inputs, default_patterns) {
        Ok(resolved) => resolved,
        Err(error) => {
            eprintln!("{NAME}: config show: {error}");
            return ExitCode::from(error.exit_code());
        }
    };

    let document = match document(&resolved) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("{NAME}: config show: {error}");
            return ExitCode::from(2);
        }
    };

    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(document.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{NAME}: config show: writing to stdout: {err}");
            ExitCode::from(2)
        }
    }
}

/// The header comment and the serialized config.
///
/// The header is a comment so the whole output still loads through
/// `Config::parse`; one line, then no blank line, because a blank line between
/// the comment and the document reads like part of the header.
fn document(resolved: &resolve::Resolved) -> Result<String, toml::ser::Error> {
    let body = resolved.config.to_toml()?;
    Ok(format!("# source: {}\n{body}", resolved.source))
}

/// `config create`: write the built-in config to the resolved path and print
/// that path (spec §6).
///
/// The bytes are [`defaults_toml`]'s, untouched, so the created file is what
/// `config defaults` prints (spec §10). The path comes from the resolution
/// order of spec §3.2, so `--config` and the environment variable decide where
/// the file goes and no run depends on the machine's home directory.
pub fn config_create(flag: Option<PathBuf>) -> ExitCode {
    let inputs = Inputs::from_env(flag);
    let path = match create(&inputs) {
        Ok(path) => path,
        Err(error) => {
            // The reason and the path share stderr: a caller that reads stdout
            // for the written path must not see one at exit 2.
            eprintln!("{NAME}: config create: {error}");
            return ExitCode::from(2);
        }
    };

    let mut stdout = io::stdout().lock();
    // Only the path, so `$(config create)` is usable as the file name.
    match writeln!(stdout, "{}", path.display()).and_then(|()| stdout.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!(
                "{NAME}: config create: {}: writing to stdout: {err}",
                path.display()
            );
            ExitCode::from(2)
        }
    }
}

/// Why `config create` refused. Every variant is exit 2 (spec §6).
#[derive(Debug)]
enum CreateError {
    /// Neither `--config`, nor the environment variable, nor the default path
    /// names a file, so there is nowhere to write.
    NoPath,
    /// The target exists. spec §6: the file is left byte-identical.
    Exists { path: PathBuf },
    /// Creating a directory, writing the temporary file, or claiming the target
    /// name failed.
    Io { path: PathBuf, source: io::Error },
}

impl std::fmt::Display for CreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateError::NoPath => write!(
                f,
                "no config path: pass --config <path> or set {}",
                resolve::ENV_CONFIG
            ),
            CreateError::Exists { path } => write!(
                f,
                "{}: already exists; edit it, move it, or remove it",
                path.display()
            ),
            CreateError::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

/// Write the defaults at the resolved path and return the absolute path.
///
/// The steps are ordered so that nothing is created when the command is going
/// to refuse: the target is checked first, then the directories, then the file.
fn create(inputs: &Inputs) -> Result<PathBuf, CreateError> {
    let path = resolve::target_path(inputs).ok_or(CreateError::NoPath)?;
    // spec §6 prints the absolute path; a relative `--config` is resolved
    // against the working directory. `std::path::absolute` is used rather than
    // `canonicalize` because the file does not exist yet, and because it must
    // not rewrite the path through symbolic links the user did not name.
    let path = std::path::absolute(&path).map_err(|source| CreateError::Io {
        path: path.clone(),
        source,
    })?;

    check_absent(&path)?;

    // `absolute` leaves the path below the working directory for a bare file
    // name, so the parent is always present; the fallback keeps that implicit.
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| CreateError::Io {
        path: parent.to_path_buf(),
        source,
    })?;

    write_atomically(&path)?;
    Ok(path)
}

/// The fast refusal, which also keeps `create` from calling `create_dir_all`
/// for a user who already has a config. The publish below is what actually
/// enforces spec §6, including against a concurrent run.
///
/// `symlink_metadata` rather than `exists`, so a dangling symbolic link counts
/// as occupied: writing through it would put the file outside the named path.
fn check_absent(path: &Path) -> Result<(), CreateError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(CreateError::Exists {
            path: path.to_path_buf(),
        }),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CreateError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Write the defaults through a temporary name in the target's own directory,
/// then claim the target name.
///
/// Same directory, so the publish cannot cross a file system boundary. A run
/// that dies mid-write leaves the target absent, never half-written: the bytes
/// become visible only when the whole file exists under the temp name.
///
/// This is the temp-file-and-rename shape the ticket asks for, with the publish
/// step done as a hard link. That is a deliberate deviation: `fs::rename`
/// replaces an existing target on Unix (`rename(2)`) and on Windows
/// (`MOVEFILE_REPLACE_EXISTING`), so with a rename the target's existence would
/// depend only on the check above, which a concurrent run can race. The link
/// keeps the same intent and adds the guarantee.
fn write_atomically(path: &Path) -> Result<(), CreateError> {
    let temp = temp_path(path)?;
    let bytes = defaults_toml();

    let published = write_temp(&temp, bytes.as_bytes(), path).and_then(|()| publish(&temp, path));
    if published.is_err() {
        // The target is untouched on every failure path, so drop the temporary
        // file rather than leave a stray file beside a user's config.
        let _ = fs::remove_file(&temp);
    }
    published
}

fn temp_path(path: &Path) -> Result<PathBuf, CreateError> {
    let name = path.file_name().ok_or_else(|| CreateError::Io {
        path: path.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "the path names no file"),
    })?;

    let mut temp = name.to_os_string();
    // The process id keeps a temporary file from an interrupted run from
    // blocking the next one, and `create_new` below keeps two live runs apart.
    temp.push(format!(".{}.tmp", std::process::id()));
    Ok(path.with_file_name(temp))
}

fn write_temp(temp: &Path, bytes: &[u8], path: &Path) -> Result<(), CreateError> {
    let io_error = |target: &Path, source| CreateError::Io {
        path: target.to_path_buf(),
        source,
    };

    // `create_new` refuses a name that already exists, so a concurrent run is
    // reported instead of silently reused. The error names the temporary file,
    // because that is the name already taken; the name derives from the target,
    // so the message still points at the config.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp)
        .map_err(|source| io_error(temp, source))?;

    file.write_all(bytes)
        .map_err(|source| io_error(path, source))?;
    // `sync_all` before the publish is what makes "the file is there" mean "the
    // bytes are on disk", so a crash after the publish cannot truncate it.
    file.sync_all().map_err(|source| io_error(path, source))
}

/// Give the temp file's bytes the target name, failing when the target exists.
///
/// Not `fs::rename`: it replaces an existing target on Unix (`rename(2)`) and
/// on Windows (`MOVEFILE_REPLACE_EXISTING`), so it cannot carry spec §6's
/// "never overwrites". A hard link fails with `AlreadyExists` on both, and is
/// atomic, so the target name either stays absent or names the complete file.
fn publish(temp: &Path, path: &Path) -> Result<(), CreateError> {
    match fs::hard_link(temp, path) {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            return Err(CreateError::Exists {
                path: path.to_path_buf(),
            });
        }
        Err(source) => {
            return Err(CreateError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    }

    // The target holds the complete bytes from here on, so a failure to drop
    // the second name does not make the command a failure.
    let _ = fs::remove_file(temp);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fails every write, standing in for a broken pipe or a full disk.
    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "no reader"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "no reader"))
        }
    }

    /// One target resolved, the other skipped, which is the state a single
    /// `--author` flag outside a repository produces.
    fn half_skipped() -> identity::Identity {
        identity::Identity {
            author: identity::Value::Resolved("A (aider)".to_owned()),
            committer: identity::Value::OutsideRepository,
        }
    }

    #[test]
    fn a_skipped_identity_target_is_reported_once_per_target() {
        let skipped = identity::Identity {
            author: identity::Value::OutsideRepository,
            committer: identity::Value::OutsideRepository,
        };

        let notes = skip_notes(&skipped);

        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(notes[0].contains("skipping author"), "{notes:?}");
        assert!(notes[1].contains("skipping committer"), "{notes:?}");
        for note in &notes {
            assert!(note.contains("not inside a repository"), "{note}");
            assert!(note.starts_with(NAME), "a hook log names the tool: {note}");
        }
    }

    #[test]
    fn a_resolved_identity_target_is_not_reported() {
        // spec §5.2: the note exists for the skip. A normal run inside a
        // repository says nothing, so `-v` adds no noise to the common case.
        let resolved = identity::Identity {
            author: identity::Value::Resolved("A <a@example.com>".to_owned()),
            committer: identity::Value::Resolved("A <a@example.com>".to_owned()),
        };

        assert!(skip_notes(&resolved).is_empty());
        assert_eq!(
            skip_notes(&half_skipped()),
            [format!(
                "{NAME}: verify: skipping committer: not inside a repository"
            )],
            "the note names the skipped target and the reason, and nothing else"
        );
    }

    #[test]
    fn stdout_bytes_are_exactly_the_serializer_output() {
        // The point of the ticket: the command adds no formatting, so its bytes
        // are `defaults_toml()` and cannot drift from `config create`.
        let mut written = Vec::new();
        write_defaults(&mut written).expect("a Vec write cannot fail");

        assert_eq!(written, defaults_toml().into_bytes());
    }

    #[test]
    fn a_failed_write_is_returned_rather_than_ignored() {
        let err = write_defaults(&mut FailingWriter).expect_err("the writer fails");

        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    }

    fn resolved(source: resolve::Source, text: &str) -> resolve::Resolved {
        resolve::Resolved {
            source,
            config: crate::config::Config::parse(text).expect("the fixture parses"),
        }
    }

    #[test]
    fn show_names_the_source_path_in_one_header_comment() {
        let path = PathBuf::from("/etc/commit-msg-attribution-checker/config.toml");
        let resolved = resolved(
            resolve::Source::File(path.clone()),
            "version = 1\n\n[[pattern]]\nid = \"p\"\ndescription = \"d\"\ntarget = \"line\"\nregex = \"x\"\n",
        );

        let document = document(&resolved).expect("the model serializes");
        let expected = format!("# source: {}", path.display());

        assert_eq!(
            document.lines().next(),
            Some(expected.as_str()),
            "{document}"
        );
        assert_eq!(
            document
                .lines()
                .filter(|line| line.starts_with('#'))
                .count(),
            1,
            "exactly one header line:\n{document}"
        );
    }

    #[test]
    fn show_headers_the_built_in_defaults() {
        let resolved = resolve::Resolved {
            source: resolve::Source::BuiltInDefaults,
            config: crate::defaults::default_patterns(),
        };

        let document = document(&resolved).expect("the model serializes");

        assert_eq!(document.lines().next(), Some("# source: built-in defaults"));
        // The defaults carry `version = 1`, so the fallback case emits it
        // without the command inventing one (spec §3.4).
        assert!(document.contains("\nversion = 1\n"), "{document}");
        assert_eq!(
            document.matches("[[pattern]]").count(),
            crate::defaults::default_patterns().patterns.len()
        );
    }

    #[test]
    fn show_body_is_the_model_not_the_file_text() {
        // The point of re-serializing: a file that loads with odd formatting
        // must come back canonical, or `config show` would just be `cat`.
        let text = "  version=1 # mine\n\n[[pattern]]\n  target = 'subject'\n id='p'\n  regex = '^a'\n description = 'd'\n";
        let resolved = resolved(resolve::Source::BuiltInDefaults, text);

        let document = document(&resolved).expect("the model serializes");
        let body = document.lines().skip(1).collect::<Vec<_>>().join("\n");

        assert!(body.starts_with("version = 1\n"), "{document}");
        assert!(!body.contains("# mine"), "{document}");
        assert!(!body.contains("target = '"), "{document}");
        crate::config::Config::parse(&document).expect("the output loads back");
    }

    #[test]
    fn show_of_an_empty_pattern_list_keeps_only_the_header() {
        // spec §3.3: no `[[pattern]]` entry disables all checking, so this is a
        // real configuration and the output must still load.
        let resolved = resolved(resolve::Source::BuiltInDefaults, "version = 1\n");

        let document = document(&resolved).expect("the model serializes");

        assert!(!document.contains("[[pattern]]"), "{document}");
        let reloaded = crate::config::Config::parse(&document)
            .unwrap_or_else(|err| panic!("{err}\n{document}"));
        assert!(reloaded.patterns.is_empty());
    }

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("cmac-create-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("creating the scratch directory");
        path
    }

    #[test]
    fn creates_the_file_and_the_parent_directories() {
        let dir = scratch("parents");
        let target = dir.join("a/b/c/config.toml");
        let inputs = Inputs {
            flag: Some(target.clone()),
            ..Inputs::default()
        };

        let returned = create(&inputs).expect("a fresh path is written");

        let text = fs::read_to_string(&target).expect("the file exists");
        assert_eq!(
            text,
            defaults_toml(),
            "spec §10: one serializer for the printed and the written bytes"
        );
        // The bytes are only useful if the binary can load them back, which is
        // what a later `verify` will do.
        crate::config::Config::parse(&text).expect("the created file is a valid config");
        assert!(
            returned.is_absolute(),
            "{}: not absolute",
            returned.display()
        );
        assert_eq!(fs::read_dir(dir.join("a/b/c")).unwrap().count(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_without_leaving_a_temporary_file() {
        // The temp name lives beside the target, so a leak would show up here as
        // a second entry and would confuse a user editing the config.
        let dir = scratch("temp");
        let inputs = Inputs {
            flag: Some(dir.join("config.toml")),
            ..Inputs::default()
        };

        create(&inputs).expect("a fresh path is written");
        let names: Vec<String> = fs::read_dir(&dir)
            .expect("reading the scratch directory")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, ["config.toml"], "{names:?}");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_an_existing_target_and_leaves_it_identical() {
        // spec §8 criterion 10, at the function level; the spawned-binary test
        // in tests/config_create.rs asserts the exit status and the stream.
        let dir = scratch("exists");
        let target = dir.join("config.toml");
        let original =
            "[[pattern]]\nid = \"kept\"\ndescription = \"d\"\ntarget = \"line\"\nregex = \"x\"\n";
        fs::write(&target, original).expect("writing the existing config");
        let inputs = Inputs {
            flag: Some(target.clone()),
            ..Inputs::default()
        };

        let error = create(&inputs).expect_err("an existing target is refused");

        assert!(matches!(error, CreateError::Exists { .. }), "{error}");
        assert_eq!(fs::read_to_string(&target).unwrap(), original);
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            1,
            "no temp is left behind"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_when_the_target_name_is_a_directory() {
        // An occupied name is enough to refuse; writing would need the name to
        // become a file, which the user did not ask for.
        let dir = scratch("occupied");
        let target = dir.join("config.toml");
        fs::create_dir(&target).expect("creating the directory");
        let inputs = Inputs {
            flag: Some(target.clone()),
            ..Inputs::default()
        };

        let error = create(&inputs).expect_err("an occupied name is refused");

        assert!(matches!(error, CreateError::Exists { .. }), "{error}");
        assert!(target.is_dir(), "the directory is left alone");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_resolvable_path_is_an_error() {
        // Nothing names a file, so there is nowhere to write, and the message
        // names the inputs that would.
        let error = create(&Inputs::default()).expect_err("no path is an error");

        assert!(matches!(error, CreateError::NoPath), "{error}");
        assert!(error.to_string().contains(resolve::ENV_CONFIG), "{error}");
    }

    #[test]
    fn a_relative_path_is_resolved_against_the_working_directory() {
        let relative = PathBuf::from(format!("cmac-relative-{}.toml", std::process::id()));
        let absolute = std::path::absolute(&relative).expect("a working directory exists");
        let _ = fs::remove_file(&absolute);
        let inputs = Inputs {
            flag: Some(relative),
            ..Inputs::default()
        };

        let returned = create(&inputs).expect("a fresh relative path is written");

        assert_eq!(returned, absolute);
        assert!(
            returned.is_absolute(),
            "{}: not absolute",
            returned.display()
        );
        assert_eq!(fs::read_to_string(&returned).unwrap(), defaults_toml());

        let _ = fs::remove_file(&absolute);
    }
}
