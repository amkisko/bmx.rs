use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

use super::util::{bmx, init_make_repo};

#[test]
fn cli_without_args_fails_with_helpful_message() {
    let home = TempDir::new().expect("home tempdir");
    bmx(home.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("provide a command"));
}

#[test]
fn global_flag_requires_trust_flag() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let repo = init_make_repo(repos.path(), "global-flag", "global-flag", "OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["--global", "install", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains("`--global` requires `--trust`"));
}

#[test]
fn source_set_and_show_work() {
    let home = TempDir::new().expect("home tempdir");

    bmx(home.path())
        .args(["source", "set-default", "github.com/my-org/"])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://github.com/my-org"));

    bmx(home.path())
        .args(["source", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://github.com/my-org"));
}

#[test]
fn isolation_set_and_show_work() {
    let home = TempDir::new().expect("home tempdir");

    bmx(home.path())
        .args(["isolation", "set-default", "auto"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "default build isolation set to auto",
        ));

    bmx(home.path())
        .args(["isolation", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto"));
}

#[test]
fn checkout_set_and_show_work() {
    let home = TempDir::new().expect("home tempdir");

    bmx(home.path())
        .args(["checkout", "set-default", "git"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "default checkout backend set to git",
        ));

    bmx(home.path())
        .args(["checkout", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("git"));
}

#[test]
fn doctor_runs_and_reports_tools() {
    let home = TempDir::new().expect("home tempdir");

    bmx(home.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bmx home:"))
        .stdout(predicate::str::contains("default source:"))
        .stdout(predicate::str::contains("build isolation:"))
        .stdout(predicate::str::contains("checkout backend:"))
        .stdout(predicate::str::contains("integrity_check"))
        .stdout(predicate::str::contains("cargo:"));
}

#[test]
fn update_without_install_fails() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let repo = init_make_repo(repos.path(), "no-install", "no-install", "x");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["update", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not installed"));
}
