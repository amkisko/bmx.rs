use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

use super::util::{app_id_for_input, bmx, init_make_repo};

#[test]
fn help_documents_no_input_flag() {
    let home = TempDir::new().expect("home tempdir");
    bmx(home.path())
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-input"));
}

#[test]
fn history_json_emits_parseable_entries() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let repo = init_make_repo(repos.path(), "json-hist", "json-hist", "JSON_HIST_OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path()).args(["install", &app]).assert().success();

    let output = bmx(home.path())
        .args(["history", "--json", "-n", "5"])
        .output()
        .expect("run history --json");
    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    let value: Value = serde_json::from_str(&stdout).expect("history --json must be JSON");
    let entries = value.as_array().expect("history --json array");
    assert!(
        entries
            .iter()
            .any(|entry| entry.get("kind").and_then(Value::as_str) == Some("install")),
        "expected an install entry in {stdout}"
    );
}

#[test]
fn doctor_json_emits_home_and_tools() {
    let home = TempDir::new().expect("home tempdir");
    let output = bmx(home.path())
        .args(["doctor", "--json"])
        .output()
        .expect("run doctor --json");
    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    let value: Value = serde_json::from_str(&stdout).expect("doctor --json must be JSON");
    assert!(value.get("home").and_then(Value::as_str).is_some());
    assert!(value.get("tools").and_then(Value::as_object).is_some());
}

#[test]
fn trust_list_json_emits_default_rule() {
    let home = TempDir::new().expect("home tempdir");
    let output = bmx(home.path())
        .args(["trust", "list", "--json"])
        .output()
        .expect("run trust list --json");
    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    let value: Value = serde_json::from_str(&stdout).expect("trust list --json must be JSON");
    assert!(value.get("default").is_some());
    assert!(value.get("rules").and_then(Value::as_array).is_some());
}

#[test]
fn uninstall_without_force_fails_when_non_interactive() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let repo = init_make_repo(repos.path(), "force-me", "force-me", "FORCE_OK");
    let app = format!("file://{}", repo.display());
    let app_id = app_id_for_input(&app);

    bmx(home.path()).args(["install", &app]).assert().success();

    bmx(home.path())
        .args(["uninstall", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--force"));

    assert!(
        home.path().join(".bmx/apps").join(&app_id).exists(),
        "uninstall without --force must not delete the app"
    );
}

#[test]
fn uninstall_force_removes_app() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let repo = init_make_repo(repos.path(), "force-ok", "force-ok", "FORCE_OK");
    let app = format!("file://{}", repo.display());
    let app_id = app_id_for_input(&app);

    bmx(home.path()).args(["install", &app]).assert().success();

    bmx(home.path())
        .args(["uninstall", "--force", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));

    assert!(
        !home.path().join(".bmx/apps").join(app_id).exists(),
        "uninstall --force should remove the app directory"
    );
}
