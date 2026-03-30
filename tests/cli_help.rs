use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn shows_help_for_top_level_command() {
    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("aienv"))
        .stdout(predicate::str::contains("exec"))
        .stdout(predicate::str::contains("shell"));
}
