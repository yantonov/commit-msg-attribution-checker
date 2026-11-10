use std::process::Command;

#[test]
fn config_without_subcommand_lists_subcommands_on_stderr_and_exits_2() {
    let output = Command::new(env!("CARGO_BIN_EXE_commit-msg-attribution-checker"))
        .arg("config")
        .output()
        .expect("running the binary");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "stdout must stay empty");
    let stderr = String::from_utf8(output.stderr).expect("stderr is utf-8");
    for subcommand in ["defaults", "create", "show"] {
        assert!(
            stderr.contains(subcommand),
            "stderr names {subcommand}: {stderr}"
        );
    }
}
