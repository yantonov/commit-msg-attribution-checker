//! Guards the harness files themselves. Every later ticket is verified with
//! `make check`, so a `TODO` placeholder that survives in `AGENTS.md` or
//! `Makefile` would silently weaken that verification.

use std::fs;
use std::path::Path;

fn read(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
}

fn todos(name: &str) -> Vec<String> {
    read(name)
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("TODO"))
        .map(|(index, line)| format!("{}: {line}", index + 1))
        .collect()
}

#[test]
fn agents_md_has_no_unresolved_placeholder() {
    let found = todos("AGENTS.md");
    assert!(
        found.is_empty(),
        "AGENTS.md still holds a TODO placeholder:\n{}",
        found.join("\n")
    );
}

#[test]
fn makefile_has_no_unresolved_placeholder() {
    let found = todos("Makefile");
    assert!(
        found.is_empty(),
        "Makefile still holds a TODO placeholder:\n{}",
        found.join("\n")
    );
}

#[test]
fn agents_md_stays_within_the_line_budget() {
    let lines = read("AGENTS.md").lines().count();
    assert!(
        lines <= 150,
        "AGENTS.md has {lines} lines; move detail to docs/ before it exceeds 150"
    );
}
