use std::fs;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

use super::util::{app_id_for_input, bmx, git_stdout, init_make_repo, run_git};

#[test]
fn install_exec_update_and_uninstall_explicit_file_url() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");

    let repo = init_make_repo(repos.path(), "hello-tool", "hello-tool", "HELLO_FROM_BMX");
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
        .stdout(predicate::str::contains("HELLO_FROM_BMX"));

    bmx(home.path())
        .args(["update", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("updated"));

    bmx(home.path())
        .args(["update"])
        .assert()
        .success()
        .stdout(predicate::str::contains("updated installed apps"));

    let install_meta = home
        .path()
        .join(".bmx/apps")
        .join(&app_id)
        .join("install.toml");
    assert!(install_meta.exists(), "install metadata should exist");

    bmx(home.path())
        .args(["uninstall", "--force", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));

    assert!(
        !home.path().join(".bmx/apps").join(app_id).exists(),
        "app directory should be removed after uninstall"
    );
}

#[test]
fn rebuild_with_install_installs_and_updates_runnable_binary() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let repo = init_make_repo(
        repos.path(),
        "rebuild-install",
        "rebuild-install",
        "REBUILD_OK",
    );
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["rebuild", "--install", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("rebuilt and installed"));

    bmx(home.path())
        .args(["exec", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("REBUILD_OK"));
}

#[test]
fn rebuild_without_install_can_build_exact_requested_version() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let repo = init_make_repo(repos.path(), "rebuild-version", "rebuild-version", "V1");
    run_git(&repo, &["tag", "v1.0.0"]);
    fs::write(
        repo.join("Makefile"),
        "all:\n\tmkdir -p bin\n\tprintf '#!/bin/sh\\necho V2\\n' > bin/rebuild-version\n\tchmod +x bin/rebuild-version\n",
    )
    .expect("write v2 makefile");
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "v2"]);
    let v100_sha = git_stdout(&repo, &["rev-list", "-n", "1", "v1.0.0"]);
    let app = format!("file://{}", repo.display());

    bmx(home.path()).args(["install", &app]).assert().success();

    bmx(home.path())
        .args(["rebuild", &format!("{app}@{v100_sha}")])
        .assert()
        .success()
        .stdout(predicate::str::contains("rebuilt"));

    let cached_repo = home.path().join(".bmx/apps/rebuild-version/repo");
    let head = git_stdout(&cached_repo, &["rev-parse", "HEAD"]);
    assert_eq!(head, v100_sha);
}
