use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

use super::util::{bmx, init_make_repo};

#[cfg(unix)]
#[test]
fn shim_init_path_and_add_generate_executable_stub() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "shim-app", "shim-app", "SHIM_OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["shim", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PATH="));

    bmx(home.path()).args(["shim", "init"]).assert().success();

    bmx(home.path()).args(["install", &app]).assert().success();

    bmx(home.path())
        .args(["shim", "add", &app])
        .assert()
        .success();

    let shim = home.path().join(".bmx/shims/shim-app");
    assert!(shim.is_file(), "shim file exists");
    let st = Command::new("sh")
        .current_dir(repos.path())
        .env("HOME", home.path())
        .arg(&shim)
        .arg("x")
        .stdout(std::process::Stdio::null())
        .status()
        .expect("run shim");
    assert!(st.success(), "shim should execute bmx exec");
}

#[test]
fn rm_flag_with_shim_does_not_create_persistent_home_state() {
    let home = TempDir::new().expect("home");

    bmx(home.path())
        .args(["--rm", "shim", "init"])
        .assert()
        .success();

    assert!(
        !home.path().join(".bmx").exists(),
        "--rm must not create a persistent ~/.bmx under HOME"
    );
}
