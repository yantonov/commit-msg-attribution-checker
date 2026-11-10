//! Config resolution (spec §3.2): the lookup that decides which pattern list
//! runs.
//!
//! The environment reaches this module only through [`Inputs`], so no test has
//! to touch process globals and no result depends on the home directory of the
//! machine that runs it.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::config::{Config, ConfigError};

/// Environment variable naming a config file (spec §3.1).
pub(crate) const ENV_CONFIG: &str = "COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG";

/// The path below the config directory (spec §3.1). Kept as one literal so the
/// resolved path is exactly the path the spec names.
const LOCAL_PATH: &str = "commit-msg-attribution-checker/config.toml";

/// Everything resolution reads from outside the process: the flag and the
/// three environment inputs. Injected rather than read so that tests state
/// their inputs instead of inheriting the machine's.
#[derive(Debug, Clone, Default)]
pub struct Inputs {
    /// `--config <path>` (spec §3.1).
    pub flag: Option<PathBuf>,
    /// `COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG`.
    pub env_var: Option<PathBuf>,
    /// `XDG_CONFIG_HOME`, used verbatim when set (spec §3.1).
    pub xdg_config_home: Option<PathBuf>,
    /// The home directory behind the `~/.config` fallback.
    pub home: Option<PathBuf>,
}

impl Inputs {
    /// Read the real process environment.
    ///
    /// `dirs` supplies only the home directory. Its config-directory helper
    /// returns `%APPDATA%` on Windows, which contradicts the literal path in
    /// spec §3.1, so the path is assembled here instead.
    pub fn from_env(flag: Option<PathBuf>) -> Self {
        Self {
            flag,
            env_var: std::env::var_os(ENV_CONFIG).map(PathBuf::from),
            // An empty variable is unset for the XDG spec, so it must not
            // produce a relative path silently rooted in the working directory.
            xdg_config_home: std::env::var_os("XDG_CONFIG_HOME")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            home: dirs::home_dir(),
        }
    }
}

/// Which input supplied the pattern list, so `config show` can name it in the
/// header comment (spec §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A file named by `--config`.
    Flag(PathBuf),
    /// A file named by `COMMIT_MSG_ATTRIBUTION_CHECKER_CONFIG`.
    EnvVar(PathBuf),
    /// The file found at the default path.
    File(PathBuf),
    /// No file applies, so the compiled-in list runs (spec §3.2 step 4).
    BuiltInDefaults,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Flag(path) | Source::EnvVar(path) | Source::File(path) => {
                write!(f, "{}", path.display())
            }
            Source::BuiltInDefaults => f.write_str("built-in defaults"),
        }
    }
}

/// The pattern list that `verify` will apply, with where it came from.
#[derive(Debug)]
pub struct Resolved {
    pub source: Source,
    pub config: Config,
}

/// A config that cannot be loaded. Every variant maps to exit status 2
/// (spec §3.2, §6).
#[derive(Debug)]
pub enum LoadError {
    /// The file could not be read: absent, a directory, or a permission error.
    Read { path: PathBuf, source: io::Error },
    /// The file was read but is not a usable config.
    Invalid { path: PathBuf, source: ConfigError },
}

impl LoadError {
    /// spec §6: an I/O or config error exits 2.
    pub fn exit_code(&self) -> u8 {
        2
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Read { path, source } => write!(f, "{}: {source}", path.display()),
            LoadError::Invalid { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

/// The default config path from spec §3.1, or `None` when the environment names
/// neither `XDG_CONFIG_HOME` nor a home directory.
pub fn default_path(inputs: &Inputs) -> Option<PathBuf> {
    let base = match &inputs.xdg_config_home {
        Some(dir) => dir.clone(),
        None => inputs.home.clone()?.join(".config"),
    };
    Some(base.join(LOCAL_PATH))
}

/// The path named by the first of `--config`, the environment variable and the
/// default path (spec §3.2 steps 1 to 3).
///
/// This is the path half of [`resolve`], for `config create`, which normally
/// runs while the file does not exist yet. It cannot use [`resolve`]: there a
/// missing file at an explicit path is an error, and a missing file at the
/// default path selects the built-in list, so neither returns the path.
pub fn target_path(inputs: &Inputs) -> Option<PathBuf> {
    inputs
        .flag
        .clone()
        .or_else(|| inputs.env_var.clone())
        .or_else(|| default_path(inputs))
}

/// Resolve and load the config in the order of spec §3.2.
///
/// `--config`, then the environment variable, then the default path, then the
/// built-in defaults. The first two are explicit: a missing or malformed file
/// there is an error, never a fallback to the defaults, because a user who
/// named a path must not be checked against a list they did not choose.
///
/// The defaults are a closure so the caller can build them only when step 4 is
/// reached, and so this module owns no pattern records.
pub fn resolve(inputs: &Inputs, defaults: impl FnOnce() -> Config) -> Result<Resolved, LoadError> {
    if let Some(path) = &inputs.flag {
        return load(path, Source::Flag(path.clone()));
    }

    if let Some(path) = &inputs.env_var {
        return load(path, Source::EnvVar(path.clone()));
    }

    if let Some(path) = default_path(inputs) {
        match fs::read_to_string(&path) {
            Ok(text) => {
                let config = Config::parse(&text).map_err(|source| LoadError::Invalid {
                    path: path.clone(),
                    source,
                })?;
                return Ok(Resolved {
                    source: Source::File(path),
                    config,
                });
            }
            // Step 3 is conditional on the file existing, so an absent file
            // continues to the defaults; any other error is a real one.
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(LoadError::Read { path, source }),
        }
    }

    Ok(Resolved {
        source: Source::BuiltInDefaults,
        config: defaults(),
    })
}

fn load(path: &Path, source: Source) -> Result<Resolved, LoadError> {
    let text = fs::read_to_string(path).map_err(|error| LoadError::Read {
        path: path.to_path_buf(),
        source: error,
    })?;
    let config = Config::parse(&text).map_err(|error| LoadError::Invalid {
        path: path.to_path_buf(),
        source: error,
    })?;
    Ok(Resolved { source, config })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A scratch directory under the system temp dir: no extra crate, and no
    /// test reads or writes the real user profile.
    struct Scratch {
        path: PathBuf,
    }

    impl Scratch {
        fn new() -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("cmac-resolve-{}-{unique}", std::process::id()));
            fs::create_dir_all(&path).expect("creating the scratch directory");
            Self { path }
        }

        /// Write a file below the scratch directory and return its path.
        fn write(&self, relative: &str, text: &str) -> PathBuf {
            let path = self.path.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("creating the parent directory");
            }
            fs::write(&path, text).expect("writing the file");
            path
        }

        /// Write a valid config at the default path below `base`.
        fn write_default(&self, base: &Path, id: &str) -> PathBuf {
            let path = base.join(LOCAL_PATH);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("creating the parent directory");
            }
            fs::write(&path, config_text(id)).expect("writing the config");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn config_text(id: &str) -> String {
        format!(
            "[[pattern]]\nid = \"{id}\"\ndescription = \"d\"\ntarget = \"line\"\nregex = \"x\"\n"
        )
    }

    fn built_in() -> Config {
        Config::parse(&config_text("built-in")).expect("the defaults fixture parses")
    }

    fn resolved(inputs: &Inputs) -> Result<Resolved, LoadError> {
        resolve(inputs, built_in)
    }

    /// The loaded list as ids, so a test states which source won by naming the
    /// patterns it wrote.
    fn ids(resolved: &Resolved) -> Vec<String> {
        resolved
            .config
            .patterns
            .iter()
            .map(|pattern| pattern.id.clone())
            .collect()
    }

    fn error(inputs: &Inputs) -> LoadError {
        match resolved(inputs) {
            Ok(resolved) => panic!(
                "expected a load error, resolved to {} with {:?}",
                resolved.source,
                ids(&resolved)
            ),
            Err(error) => error,
        }
    }

    #[test]
    fn flag_wins_over_env_var_and_file() {
        let scratch = Scratch::new();
        let flag = scratch.write("flag.toml", &config_text("from-flag"));
        let env_var = scratch.write("env.toml", &config_text("from-env"));
        let xdg = scratch.path.join("xdg");
        scratch.write_default(&xdg, "from-file");

        let inputs = Inputs {
            flag: Some(flag.clone()),
            env_var: Some(env_var),
            xdg_config_home: Some(xdg),
            home: None,
        };
        let resolved = resolved(&inputs).expect("the flag path loads");

        assert_eq!(resolved.source, Source::Flag(flag));
        assert_eq!(ids(&resolved), ["from-flag"]);
    }

    #[test]
    fn env_var_wins_over_file() {
        let scratch = Scratch::new();
        let env_var = scratch.write("env.toml", &config_text("from-env"));
        let xdg = scratch.path.join("xdg");
        scratch.write_default(&xdg, "from-file");

        let inputs = Inputs {
            flag: None,
            env_var: Some(env_var.clone()),
            xdg_config_home: Some(xdg),
            home: None,
        };
        let resolved = resolved(&inputs).expect("the environment path loads");

        assert_eq!(resolved.source, Source::EnvVar(env_var));
        assert_eq!(ids(&resolved), ["from-env"]);
    }

    #[test]
    fn file_wins_over_defaults() {
        let scratch = Scratch::new();
        let xdg = scratch.path.join("xdg");
        let path = scratch.write_default(&xdg, "from-file");

        let inputs = Inputs {
            xdg_config_home: Some(xdg),
            ..Inputs::default()
        };
        let resolved = resolved(&inputs).expect("the default path loads");

        assert_eq!(resolved.source, Source::File(path));
        assert_eq!(ids(&resolved), ["from-file"]);
    }

    #[test]
    fn defaults_when_no_file_exists() {
        let scratch = Scratch::new();

        let inputs = Inputs {
            xdg_config_home: Some(scratch.path.join("nothing-here")),
            ..Inputs::default()
        };
        let resolved = resolved(&inputs).expect("defaults always resolve");

        assert_eq!(resolved.source, Source::BuiltInDefaults);
        assert_eq!(ids(&resolved), ["built-in"]);
    }

    #[test]
    fn defaults_when_the_environment_names_nothing() {
        let inputs = Inputs::default();
        let resolved = resolved(&inputs).expect("defaults always resolve");

        assert_eq!(resolved.source, Source::BuiltInDefaults);
        assert_eq!(ids(&resolved), ["built-in"]);
    }

    #[test]
    fn a_file_replaces_the_defaults_instead_of_merging() {
        let scratch = Scratch::new();
        let xdg = scratch.path.join("xdg");
        // spec §3.3: the file is the complete list, so the defaults disappear.
        let path = scratch.write_default(&xdg, "only-this");

        let inputs = Inputs {
            xdg_config_home: Some(xdg),
            ..Inputs::default()
        };
        let resolved = resolved(&inputs).expect("the default path loads");

        assert_eq!(resolved.source, Source::File(path));
        assert_eq!(ids(&resolved), ["only-this"]);
    }

    #[test]
    fn a_missing_flag_path_is_an_error_even_when_a_file_exists() {
        let scratch = Scratch::new();
        let xdg = scratch.path.join("xdg");
        scratch.write_default(&xdg, "from-file");

        let inputs = Inputs {
            flag: Some(scratch.path.join("absent.toml")),
            xdg_config_home: Some(xdg),
            ..Inputs::default()
        };
        let error = error(&inputs);

        assert_eq!(error.exit_code(), 2, "{error}");
        assert!(matches!(error, LoadError::Read { .. }), "{error}");
        assert!(error.to_string().contains("absent.toml"), "{error}");
    }

    #[test]
    fn a_missing_env_var_path_is_an_error_even_when_a_file_exists() {
        let scratch = Scratch::new();
        let xdg = scratch.path.join("xdg");
        scratch.write_default(&xdg, "from-file");

        let inputs = Inputs {
            env_var: Some(scratch.path.join("absent.toml")),
            xdg_config_home: Some(xdg),
            ..Inputs::default()
        };
        let error = error(&inputs);

        assert_eq!(error.exit_code(), 2, "{error}");
        assert!(matches!(error, LoadError::Read { .. }), "{error}");
        assert!(error.to_string().contains("absent.toml"), "{error}");
    }

    #[test]
    fn a_malformed_flag_file_is_an_error() {
        let scratch = Scratch::new();
        let flag = scratch.write("broken.toml", "not = [valid");

        let inputs = Inputs {
            flag: Some(flag.clone()),
            ..Inputs::default()
        };
        let error = error(&inputs);

        assert_eq!(error.exit_code(), 2, "{error}");
        assert!(matches!(error, LoadError::Invalid { .. }), "{error}");
        assert!(error.to_string().contains("broken.toml"), "{error}");
    }

    #[test]
    fn a_malformed_env_var_file_is_an_error() {
        let scratch = Scratch::new();
        let env_var = scratch.write("broken.toml", "not = [valid");

        let inputs = Inputs {
            env_var: Some(env_var),
            ..Inputs::default()
        };
        let error = error(&inputs);

        assert_eq!(error.exit_code(), 2, "{error}");
        assert!(matches!(error, LoadError::Invalid { .. }), "{error}");
    }

    #[test]
    fn a_malformed_file_at_the_default_path_is_an_error() {
        let scratch = Scratch::new();
        let xdg = scratch.path.join("xdg");
        let path = scratch.write_default(&xdg, "valid");
        fs::write(&path, "not = [valid").expect("overwriting the config");

        let inputs = Inputs {
            xdg_config_home: Some(xdg),
            ..Inputs::default()
        };
        let error = error(&inputs);

        assert_eq!(error.exit_code(), 2, "{error}");
        assert!(matches!(error, LoadError::Invalid { .. }), "{error}");
    }

    #[test]
    fn xdg_config_home_supplies_the_default_path() {
        let scratch = Scratch::new();
        let xdg = scratch.path.join("xdg");
        let path = scratch.write_default(&xdg, "from-xdg");
        let home = scratch.path.join("home");
        scratch.write_default(&home, "from-home");

        let inputs = Inputs {
            xdg_config_home: Some(xdg.clone()),
            home: Some(home),
            ..Inputs::default()
        };
        let resolved = resolved(&inputs).expect("the xdg file loads");

        assert_eq!(resolved.source, Source::File(path.clone()));
        assert_eq!(resolved.source.to_string(), path.display().to_string());
        assert_eq!(ids(&resolved), ["from-xdg"]);
        assert_eq!(default_path(&inputs), Some(xdg.join(LOCAL_PATH)));
    }

    #[test]
    fn an_unset_xdg_config_home_falls_back_to_home_dot_config() {
        let scratch = Scratch::new();
        let home = scratch.path.join("home");
        let path = scratch.write_default(&home.join(".config"), "from-home");

        let inputs = Inputs {
            xdg_config_home: None,
            home: Some(home.clone()),
            ..Inputs::default()
        };
        let resolved = resolved(&inputs).expect("the home file loads");

        assert_eq!(resolved.source, Source::File(path.clone()));
        assert_eq!(ids(&resolved), ["from-home"]);
        assert_eq!(
            default_path(&inputs),
            Some(home.join(".config").join(LOCAL_PATH))
        );
    }

    #[test]
    fn a_set_xdg_config_home_does_not_fall_back_to_home() {
        let scratch = Scratch::new();
        let home = scratch.path.join("home");
        scratch.write_default(&home, "from-home");

        let inputs = Inputs {
            xdg_config_home: Some(scratch.path.join("empty-xdg")),
            home: Some(home),
            ..Inputs::default()
        };
        let resolved = resolved(&inputs).expect("defaults resolve");

        assert_eq!(
            resolved.source,
            Source::BuiltInDefaults,
            "spec §3.1 names one default path, not a search path"
        );
    }

    #[test]
    fn the_default_path_never_uses_the_roaming_app_data_directory() {
        // On Windows the config-directory helper returns %APPDATA%, which
        // spec §3.1 does not name, so the path is assembled from the home
        // directory by hand.
        let home = PathBuf::from(r"C:\Users\someone");
        let inputs = Inputs {
            home: Some(home.clone()),
            ..Inputs::default()
        };

        let path = default_path(&inputs).expect("a home directory is enough");
        let rendered = path.to_string_lossy().to_lowercase();

        assert!(
            rendered.ends_with("commit-msg-attribution-checker/config.toml"),
            "{rendered}"
        );
        assert!(rendered.contains(".config"), "{rendered}");
        assert!(!rendered.contains("appdata"), "{rendered}");
        assert!(!rendered.contains("roaming"), "{rendered}");
    }

    #[test]
    fn the_source_never_calls_the_platform_config_helper() {
        // The helper is not wrong everywhere, only on Windows, so the guard is
        // on the code rather than on one platform's behaviour. Every mention
        // allowed here is a comment, and the needle is assembled so that this
        // test does not match itself.
        let needle = concat!("config", "_dir");
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut scanned = 0;

        for entry in fs::read_dir(&dir).expect("reading src/") {
            let path = entry.expect("a directory entry").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            scanned += 1;
            let text = fs::read_to_string(&path).expect("reading a source file");
            for (index, line) in text.lines().enumerate() {
                if line.contains(needle) {
                    assert!(
                        line.trim_start().starts_with("//"),
                        "{}:{}: build the config path from the home directory instead: {line}",
                        path.display(),
                        index + 1
                    );
                }
            }
        }

        assert!(scanned > 0, "the scan found no source files");
    }

    #[test]
    fn defaults_are_not_built_when_a_file_wins() {
        let scratch = Scratch::new();
        let flag = scratch.write("flag.toml", &config_text("from-flag"));

        let inputs = Inputs {
            flag: Some(flag),
            ..Inputs::default()
        };
        let resolved = resolve(&inputs, || panic!("the defaults must stay unbuilt"))
            .expect("the flag path loads");

        assert_eq!(ids(&resolved), ["from-flag"]);
    }

    #[test]
    fn the_target_path_follows_flag_then_env_var_then_default() {
        let scratch = Scratch::new();
        let flag = scratch.path.join("flag.toml");
        let env_var = scratch.path.join("env.toml");
        let xdg = scratch.path.join("xdg");
        let home = scratch.path.join("home");

        let all = Inputs {
            flag: Some(flag.clone()),
            env_var: Some(env_var.clone()),
            xdg_config_home: Some(xdg.clone()),
            home: Some(home.clone()),
        };
        assert_eq!(target_path(&all), Some(flag));

        let env_only = Inputs {
            flag: None,
            env_var: Some(env_var.clone()),
            ..all.clone()
        };
        assert_eq!(target_path(&env_only), Some(env_var));

        let default_only = Inputs {
            xdg_config_home: Some(xdg.clone()),
            home: Some(home.clone()),
            ..Inputs::default()
        };
        assert_eq!(target_path(&default_only), Some(xdg.join(LOCAL_PATH)));

        // Nothing names a path, so there is nowhere to write: `config create`
        // reports that instead of inventing a location.
        assert_eq!(target_path(&Inputs::default()), None);
    }

    #[test]
    fn display_names_the_source_the_header_comment_prints() {
        let path = PathBuf::from("/etc/commit-msg-attribution-checker/config.toml");

        assert_eq!(Source::BuiltInDefaults.to_string(), "built-in defaults");
        for source in [
            Source::Flag(path.clone()),
            Source::EnvVar(path.clone()),
            Source::File(path.clone()),
        ] {
            assert_eq!(source.to_string(), path.display().to_string());
        }
    }
}
