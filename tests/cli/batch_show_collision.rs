use std::fs;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

use super::util::{app_id_for_input, bmx, init_make_repo};

#[test]
fn update_all_then_undo_only_one_app_leaves_batch_for_the_other() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let r1 = init_make_repo(repos.path(), "batch-a", "bin-a", "A");
    let r2 = init_make_repo(repos.path(), "batch-b", "bin-b", "B");
    let app1 = format!("file://{}", r1.display());
    let app2 = format!("file://{}", r2.display());
    let id1 = app_id_for_input(&app1);
    let id2 = app_id_for_input(&app2);

    bmx(home.path()).args(["install", &app1]).assert().success();
    bmx(home.path()).args(["install", &app2]).assert().success();

    bmx(home.path()).args(["update"]).assert().success();

    let stack_path = home.path().join(".bmx/undo-stack.json");
    let stack_raw = fs::read_to_string(&stack_path).expect("undo stack");
    assert!(
        stack_raw.contains(&id1) && stack_raw.contains(&id2),
        "update-all frame should reference both app ids: {stack_raw}"
    );

    bmx(home.path())
        .args(["undo", "--only", &id1])
        .assert()
        .success()
        .stdout(predicate::str::contains("undone one install"));

    let stack_raw2 = fs::read_to_string(&stack_path).expect("undo stack");
    let frames: Vec<serde_json::Value> =
        serde_json::from_str(&stack_raw2).expect("parse undo stack");
    let last = frames.last().expect("stack non-empty");
    let items = last["items"].as_array().expect("items array");
    assert_eq!(
        items.len(),
        1,
        "top frame should have one pending rollback after partial undo: {stack_raw2}"
    );
    assert_eq!(items[0]["app_id"].as_str().expect("app_id"), id2);
    assert!(
        home.path().join(".bmx/apps").join(&id1).exists(),
        "first app should still exist after snapshot rollback"
    );
}

#[test]
fn show_lists_repo_files_and_would_remove_includes_install_metadata() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let repo = init_make_repo(repos.path(), "show-tool", "show-tool", "SHOW_OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["install", "--as", "my-show", &app])
        .assert()
        .success();

    bmx(home.path())
        .args(["show", "my-show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Makefile"))
        .stdout(predicate::str::contains("bmx.toml"));

    bmx(home.path())
        .args(["show", "my-show", "--would-remove"])
        .assert()
        .success()
        .stdout(predicate::str::contains("install.toml"))
        .stdout(predicate::str::contains("repo/Makefile"));
}

#[test]
fn install_second_source_colliding_default_id_hints_as_flag() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let r1 = init_make_repo(repos.path(), "path-a/dup-tool", "t", "A");
    let r2 = init_make_repo(repos.path(), "path-b/dup-tool", "t", "B");
    let app1 = format!("file://{}", r1.display());
    let app2 = format!("file://{}", r2.display());

    bmx(home.path()).args(["install", &app1]).assert().success();

    bmx(home.path())
        .args(["install", &app2])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--as"));
}
