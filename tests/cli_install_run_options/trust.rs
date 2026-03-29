use std::fs;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

use super::util::{bmx, bmx_no_trust_assume, init_make_repo};

#[test]
fn trust_policy_can_block_unapproved_source() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "blocked-tool", "blocked-tool", "BLOCKED");
    let app = format!("file://{}", repo.display());

    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    fs::write(
        home.path().join(".bmx/trust.toml"),
        "[default]\nallow = false\n",
    )
    .expect("write trust policy");

    bmx(home.path())
        .args(["install", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "source is blocked by trust policy",
        ));
}

#[test]
fn trust_policy_allows_explicit_prefix_when_default_denies() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "allowed-tool", "allowed-tool", "ALLOWED");
    let app = format!("file://{}", repo.display());

    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    let trust = format!(
        "[default]\nallow = false\n\n[[rules]]\nmatch_prefix = \"file://{}\"\nallow = true\n",
        repos.path().display()
    );
    fs::write(home.path().join(".bmx/trust.toml"), trust).expect("write trust policy");

    bmx(home.path()).args(["install", &app]).assert().success();
    bmx(home.path())
        .args(["exec", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("ALLOWED"));
}

#[test]
fn trust_add_key_and_show_roundtrip() {
    let home = TempDir::new().expect("home");

    bmx(home.path())
        .args([
            "trust",
            "add-key",
            "ABCD1234",
            "--match-prefix",
            "https://github.com/acme/",
        ])
        .assert()
        .success();

    bmx(home.path())
        .args(["trust", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://github.com/acme/"))
        .stdout(predicate::str::contains("ABCD1234"));
}

#[test]
fn trust_list_supports_global_and_local_filters() {
    let home = TempDir::new().expect("home");
    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    fs::write(
        home.path().join(".bmx/trust.toml"),
        "[default]\nallow = true\nallowed_signing_keys = [\"GLOBALKEY\"]\n\n[[rules]]\nmatch_prefix = \"https://github.com/acme/\"\nallow = true\nallowed_signing_keys = [\"LOCALKEY\"]\n",
    )
    .expect("write trust policy");

    bmx(home.path())
        .args(["trust", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("scope: global"))
        .stdout(predicate::str::contains("scope: local"));

    bmx(home.path())
        .args(["--global", "trust", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("scope: global"))
        .stdout(predicate::str::contains("GLOBALKEY"))
        .stdout(predicate::str::contains("scope: local").not());

    bmx(home.path())
        .args(["trust", "list", "--local"])
        .assert()
        .success()
        .stdout(predicate::str::contains("scope: local"))
        .stdout(predicate::str::contains("LOCALKEY"))
        .stdout(predicate::str::contains("scope: global").not());
}

#[test]
fn trust_list_can_filter_by_installed_app_source() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "trust-list-app", "trust-list-app", "TL");
    let app = format!("file://{}", repo.display());
    bmx(home.path()).args(["install", &app]).assert().success();

    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    let trust = format!(
        "[default]\nallow = true\n\n[[rules]]\nmatch_prefix = \"file://{}\"\nallow = true\nallowed_signing_keys = [\"MATCH\"]\n\n[[rules]]\nmatch_prefix = \"https://github.com/other/\"\nallow = true\nallowed_signing_keys = [\"OTHER\"]\n",
        repos.path().display()
    );
    fs::write(home.path().join(".bmx/trust.toml"), trust).expect("write trust policy");

    bmx(home.path())
        .args(["trust", "list", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("source:"))
        .stdout(predicate::str::contains("MATCH"))
        .stdout(predicate::str::contains("OTHER").not())
        .stdout(predicate::str::contains("effective_scope: local"));
}

#[test]
fn trust_import_repo_fails_for_unsigned_head_commit() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "unsigned-tool", "unsigned-tool", "U");
    let app = format!("file://{}", repo.display());

    bmx(home.path()).args(["install", &app]).assert().success();

    bmx(home.path())
        .args(["trust", "import-repo", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "no commit signing key/fingerprint found",
        ));
}

#[test]
fn trust_import_repo_from_source_without_install_attempts_temp_checkout() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "unsigned-src", "unsigned-src", "U");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["trust", "import-repo", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "no commit signing key/fingerprint found",
        ))
        .stderr(predicate::str::contains("not installed").not());
}

#[test]
fn install_fails_without_interactive_consent_for_untrusted_unsigned_source() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "consent-tool", "consent-tool", "C");
    let app = format!("file://{}", repo.display());

    bmx_no_trust_assume(home.path())
        .args(["install", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "untrusted source requires interactive consent",
        ));
}
