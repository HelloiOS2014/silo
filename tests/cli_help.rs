use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn shows_help_for_top_level_command() {
    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("silo"))
        .stdout(predicate::str::contains("exec"))
        .stdout(predicate::str::contains("shell"));
}
