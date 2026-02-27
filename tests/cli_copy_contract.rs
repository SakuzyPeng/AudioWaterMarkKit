#![cfg(feature = "full-cli")]

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

fn maybe_bin() -> Option<Command> {
    let Some(raw_path) = std::env::var_os("CARGO_BIN_EXE_awmkit-core") else {
        eprintln!("skip cli_copy_contract: missing CARGO_BIN_EXE_awmkit-core");
        return None;
    };
    let path = std::path::PathBuf::from(raw_path);
    if !path.is_file() {
        eprintln!(
            "skip cli_copy_contract: awmkit-core binary path is unavailable: {}",
            path.display()
        );
        return None;
    }
    Some(Command::new(path))
}

#[test]
fn quiet_verbose_conflict_has_action_in_en() {
    let Some(mut cmd) = maybe_bin() else {
        return;
    };
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
    let Some(mut cmd) = maybe_bin() else {
        return;
    };
    cmd.args(["--lang", "zh-CN", "--quiet", "--verbose", "status"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("不能同时使用 --quiet 和 --verbose"))
        .stderr(predicate::str::contains("下一步："));
}

#[test]
fn detect_without_inputs_is_actionable() {
    let Some(mut cmd) = maybe_bin() else {
        return;
    };
    cmd.args(["--lang", "en-US", "detect"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No input files were provided."))
        .stderr(predicate::str::contains("Next:"))
        .stderr(predicate::str::contains("slot_hint").not());
}

#[test]
fn evidence_clear_requires_filter_with_next_action() {
    let Some(mut cmd) = maybe_bin() else {
        return;
    };
    cmd.args(["--lang", "en-US", "evidence", "clear", "--yes"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Clear-all was refused."))
        .stderr(predicate::str::contains("Next: provide at least one filter"));
}

#[test]
fn evidence_remove_requires_yes_with_next_action() {
    let Some(mut cmd) = maybe_bin() else {
        return;
    };
    cmd.args(["--lang", "en-US", "evidence", "remove", "1"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("was not executed"))
        .stderr(predicate::str::contains("--yes"));
}

#[test]
fn status_doctor_hides_machine_style_db_lines() {
    let Some(mut cmd) = maybe_bin() else {
        return;
    };
    cmd.args(["--lang", "en-US", "status", "--doctor"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("db.mappings=").not())
        .stdout(predicate::str::contains("db.evidence=").not());
}
