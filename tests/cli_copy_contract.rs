use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

fn bin() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("awmkit-core"))
}

#[test]
fn quiet_verbose_conflict_has_action_in_en() {
    let mut cmd = bin();
    cmd.args(["--lang", "en-US", "--quiet", "--verbose", "status"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains(
            "Cannot use --quiet with --verbose",
        ))
        .stderr(predicate::str::contains("Next:"))
        .stderr(predicate::str::contains("DETAIL:").not());
}

#[test]
fn quiet_verbose_conflict_has_action_in_zh() {
    let mut cmd = bin();
    cmd.args(["--lang", "zh-CN", "--quiet", "--verbose", "status"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("不能同时使用 --quiet 和 --verbose"))
        .stderr(predicate::str::contains("下一步："));
}

#[test]
fn detect_without_inputs_is_actionable() {
    let mut cmd = bin();
    cmd.args(["--lang", "en-US", "detect"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No input files were provided."))
        .stderr(predicate::str::contains("Next:"))
        .stderr(predicate::str::contains("slot_hint").not());
}

#[test]
fn evidence_clear_requires_filter_with_next_action() {
    let mut cmd = bin();
    cmd.args(["--lang", "en-US", "evidence", "clear", "--yes"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Clear-all was refused."))
        .stderr(predicate::str::contains("Next: provide at least one filter"));
}

#[test]
fn evidence_remove_requires_yes_with_next_action() {
    let mut cmd = bin();
    cmd.args(["--lang", "en-US", "evidence", "remove", "1"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("was not executed"))
        .stderr(predicate::str::contains("--yes"));
}

#[test]
fn status_doctor_hides_machine_style_db_lines() {
    let mut cmd = bin();
    cmd.args(["--lang", "en-US", "status", "--doctor"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("db.mappings=").not())
        .stdout(predicate::str::contains("db.evidence=").not());
}
