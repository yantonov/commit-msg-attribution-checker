//! Helpers shared by more than one integration test binary.
//!
//! Cargo runs each top-level file under `tests/` as its own binary, so code
//! cannot be shared by naming a file there. A directory module is the one place
//! two binaries can reach: `mod support;` in each of them compiles this file
//! into that binary, and cargo does not treat `tests/support/mod.rs` as a target
//! of its own.
//!
//! Only the hook-script escaping lives here. Both the installed script of
//! `hook.rs` and the checkout's own hook of `dogfood.rs` must be built by the
//! same code, because two copies of the quoting would drift and a drifted copy
//! produces a hook that silently fails on the developer's path.

use std::path::Path;

/// The one hook script, with the command substituted.
///
/// spec §7's text is the template, so the documented snippet and the installed
/// one cannot differ in anything but the command word.
/// `the_readme_documents_the_installed_snippet` asserts
/// `hook_script(SPEC_COMMAND) == SPEC_SNIPPET`, which pins the template to the
/// spec text rather than to a second copy of it.
pub fn hook_script(command: &str) -> String {
    format!("#!/bin/sh\nexec {command} verify \"$1\"\n")
}

/// The binary path as a POSIX shell sees it inside the hook.
///
/// The path from `CARGO_BIN_EXE_*` holds backslashes on Windows and may hold
/// spaces. A `sh` script reads a backslash as an escape and an unquoted space as
/// a word break, so the separators become `/` (git's own `sh` accepts them on
/// Windows) and the result is double-quoted, with the characters that stay
/// special inside double quotes escaped. Replacing `\` first means the escaping
/// loop never sees one.
pub fn shell_command(path: &Path) -> String {
    let text = path.display().to_string().replace('\\', "/");
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for ch in text.chars() {
        if matches!(ch, '"' | '`' | '$') {
            quoted.push('\\');
        }
        quoted.push(ch);
    }
    quoted.push('"');
    quoted
}
