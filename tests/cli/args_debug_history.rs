use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

use super::util::{app_id_for_input, bmx, init_make_repo};

#[test]
fn debug_mode_shows_command_logs() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");

    let repo = init_make_repo(repos.path(), "debug-tool", "debug-tool", "DEBUG_TOOL_OK");
    let app = format!("file://{}", repo.display());

    let mut cmd = bmx(home.path());
    cmd.env("BMX_DEBUG", "1");
    cmd.args(["install", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("installed"))
        .stderr(predicate::str::contains(
            "[bmx][debug] run_checked: make -j",
        ));
}

#[test]
fn install_history_and_undo_removes_fresh_install() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let repo = init_make_repo(repos.path(), "undo-tool", "undo-tool", "UNDO_OK");
    let app = format!("file://{}", repo.display());
    let app_id = app_id_for_input(&app);

    bmx(home.path()).args(["install", &app]).assert().success();
    assert!(
        home.path().join(".bmx/apps").join(&app_id).exists(),
        "app dir should exist after install"
    );

    bmx(home.path())
        .args(["history", "-n", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("install"));

    bmx(home.path())
        .args(["undo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("undone"));

    assert!(
        !home.path().join(".bmx/apps").join(&app_id).exists(),
        "undo should remove a fresh install"
    );
}
