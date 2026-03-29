use std::fs;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

use super::util::{
    app_id_for_input, bmx, init_arg_echo_repo, init_make_repo, init_subdir_make_repo,
};

#[test]
fn install_and_exec_from_subfolder_workdir() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");

    let repo = init_subdir_make_repo(repos.path(), "sub-tool", "sub-tool", "SUBDIR_OK");
    let app = format!("file://{}", repo.display());
    let app_id = app_id_for_input(&app);

    bmx(home.path())
        .args(["install", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("installed"));

    bmx(home.path())
        .args(["exec", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("SUBDIR_OK"));

    let install_meta = home
        .path()
        .join(".bmx/apps")
        .join(app_id)
        .join("install.toml");
    let install_content = fs::read_to_string(install_meta).expect("read install metadata");
    assert!(
        install_content.contains("executable_rel = \"app/bin/sub-tool\""),
        "expected executable_rel to include subfolder path"
    );
}

#[test]
fn bare_app_name_resolves_from_default_source_and_reinstalls_when_binary_missing() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");

    let source_repo = init_make_repo(repos.path(), "tool-src", "tool", "TOOL_OK");
    let bare_store = repos.path().join("tool.git");
    let clone_status = Command::new("git")
        .args([
            "clone",
            "--bare",
            source_repo.to_string_lossy().as_ref(),
            bare_store.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("git clone --bare should run");
    assert!(clone_status.success(), "git clone --bare should succeed");

    let default_source = format!("file://{}", repos.path().display());
    bmx(home.path())
        .args(["source", "set-default", &default_source])
        .assert()
        .success();

    bmx(home.path())
        .args(["tool"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TOOL_OK"));

    let installed_bin = home.path().join(".bmx/apps/tool/repo/bin/tool");
    fs::remove_file(&installed_bin).expect("remove installed binary");

    bmx(home.path())
        .args(["tool"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TOOL_OK"));

    bmx(home.path())
        .args(["reinstall", "tool"])
        .assert()
        .success()
        .stdout(predicate::str::contains("reinstalled"));
}

#[test]
fn default_command_forwards_args_including_hyphen_prefixed_values() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");

    let repo = init_arg_echo_repo(repos.path(), "arg-tool", "arg-tool");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args([&app, "app-arg", "-app-arg--app-arg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("app-arg"))
        .stdout(predicate::str::contains("-app-arg--app-arg"));
}
