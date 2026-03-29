use std::fs;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

use super::util::{bmx, init_make_repo};

#[test]
fn rm_flag_uses_ephemeral_home_not_dot_bmx_under_home() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "rm-tool", "rm-tool", "RM_OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["--rm", "exec", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("RM_OK"));

    assert!(
        !home.path().join(".bmx").exists(),
        "--rm must not create a persistent ~/.bmx under HOME"
    );
}

#[test]
fn pin_resolves_app_from_bmx_pin_in_parent_directory() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "pin-app", "pin-app", "PIN_OK");
    let app = format!("file://{}", repo.display());

    let project = TempDir::new().expect("project");
    fs::create_dir_all(project.path().join(".bmx")).expect("pin dir");
    fs::write(project.path().join(".bmx/pin"), format!("{app}\n")).expect("write pin");

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PIN_OK"));
}

#[test]
fn pin_with_explicit_app_writes_cwd_pin_and_runs_exec() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "pin-write-exec", "pin-write-exec", "PW_EXEC");
    let app = format!("file://{}", repo.display());

    let project = TempDir::new().expect("project");

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("PW_EXEC"));

    let pins_path = project.path().join(".bmx/pins.toml");
    assert!(pins_path.is_file(), "expected {}", pins_path.display());
    let raw = fs::read_to_string(&pins_path).expect("read pins.toml");
    assert!(
        raw.contains(&app),
        "pins.toml should contain spec; got:\n{raw}"
    );
}

#[test]
fn pin_with_explicit_app_writes_cwd_pin_default_command() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "pin-write-def", "pin-write-def", "PW_DEF");
    let app = format!("file://{}", repo.display());

    let project = TempDir::new().expect("project");

    bmx(home.path())
        .current_dir(project.path())
        .args([&app, "--pin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PW_DEF"));

    let pins_path = project.path().join(".bmx/pins.toml");
    let raw = fs::read_to_string(&pins_path).expect("read pins.toml");
    assert!(
        raw.contains(&app),
        "pins.toml should contain spec; got:\n{raw}"
    );
}

#[test]
fn pin_second_app_merges_pins_toml_and_keeps_default() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let r1 = init_make_repo(repos.path(), "multi-a", "multi-a", "MULTI_A");
    let r2 = init_make_repo(repos.path(), "multi-b", "multi-b", "MULTI_B");
    let app_a = format!("file://{}", r1.display());
    let app_b = format!("file://{}", r2.display());

    let project = TempDir::new().expect("project");

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin", &app_a])
        .assert()
        .success()
        .stdout(predicate::str::contains("MULTI_A"));

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin", &app_b])
        .assert()
        .success()
        .stdout(predicate::str::contains("MULTI_B"));

    let pins_path = project.path().join(".bmx/pins.toml");
    let raw = fs::read_to_string(&pins_path).expect("read pins.toml");
    assert!(raw.contains(&app_a), "first spec preserved:\n{raw}");
    assert!(raw.contains(&app_b), "second spec present:\n{raw}");

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MULTI_A"));

    fs::write(
        &pins_path,
        fs::read_to_string(&pins_path)
            .expect("read")
            .replace("default = \"multi-a\"", "default = \"multi-b\""),
    )
    .expect("set default");

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MULTI_B"));
}

#[test]
fn verbose_flag_prints_resolution_line_to_stderr() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "verb-tool", "verb-tool", "VERB_OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["-v", "exec", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("VERB_OK"))
        .stderr(predicate::str::contains("[bmx] app="))
        .stderr(predicate::str::contains("executable="));
}
