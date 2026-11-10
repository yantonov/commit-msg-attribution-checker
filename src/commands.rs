//! Command implementations (spec §6).

use std::io::{self, Write};
use std::process::ExitCode;

use crate::defaults::defaults_toml;

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
}
