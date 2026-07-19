use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

use super::util::bmx;

#[test]
fn version_flag_prints_package_version() {
    let home = TempDir::new().expect("home tempdir");
    bmx(home.path())
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn cli_without_args_prints_concise_help_with_examples() {
    let home = TempDir::new().expect("home tempdir");
    bmx(home.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Examples:"))
        .stderr(predicate::str::contains("bmx --help"))
        .stderr(predicate::str::contains("github.com/amkisko/bmx.rs"));
}

#[test]
fn near_subcommand_typo_suggests_instead_of_cloning() {
    let home = TempDir::new().expect("home tempdir");
    bmx(home.path())
        .arg("instal")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Did you mean `install`"))
        .stderr(predicate::str::contains("git clone").not());

    assert!(
        !home.path().join(".bmx/apps/instal").exists(),
        "typo must not create an install cache directory"
    );
}

#[test]
fn help_includes_examples_and_docs_link() {
    let home = TempDir::new().expect("home tempdir");
    bmx(home.path())
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Examples:"))
        .stdout(predicate::str::contains("bmx ripgrep"))
        .stdout(predicate::str::contains("github.com/amkisko/bmx.rs"));
}

#[test]
fn completions_bash_generates_script() {
    let home = TempDir::new().expect("home tempdir");
    bmx(home.path())
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bmx"));
}

#[test]
fn usage_error_uses_exit_code_two() {
    let home = TempDir::new().expect("home tempdir");
    bmx(home.path()).args(["exec"]).assert().failure().code(2);
}
