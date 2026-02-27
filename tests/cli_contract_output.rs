use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

fn bin() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("awmkit-core"))
}

#[test]
fn quiet_verbose_conflict_is_user_facing() {
    let mut cmd = bin();
    cmd.args(["--lang", "en-US", "--quiet", "--verbose", "status"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains(
            "--quiet and --verbose cannot be used together",
        ))
        .stderr(predicate::str::contains("DETAIL:").not());
}

#[test]
fn invalid_slot_error_uses_slot_range_message() {
    let mut cmd = bin();
    cmd.args(["--lang", "en-US", "key", "slot", "use", "99"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid value '99'"))
        .stderr(predicate::str::contains("槽位"));
}

#[test]
fn evidence_clear_requires_filter_even_with_yes() {
    let mut cmd = bin();
    cmd.args(["--lang", "en-US", "evidence", "clear", "--yes"]);
    cmd.assert().failure().stderr(predicate::str::contains(
        "refusing to clear all evidence; provide at least one filter",
    ));
}

#[test]
fn evidence_remove_requires_yes_confirmation() {
    let mut cmd = bin();
    cmd.args(["--lang", "en-US", "evidence", "remove", "1"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("requires --yes confirmation"));
}

#[test]
fn detect_without_inputs_reports_user_message() {
    let mut cmd = bin();
    cmd.args(["--lang", "en-US", "detect"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("no input files provided"))
        .stderr(predicate::str::contains("DIAG").not());
}
