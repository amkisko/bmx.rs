use std::fs;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

use super::util::{app_id_for_file_url, bmx, init_make_repo, init_repo_with_post_install, run_git};

#[test]
fn post_install_hook_runs_after_install() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_repo_with_post_install(
        repos.path(),
        "hook-tool",
        "hook-tool",
        "HOOK_OK",
        "touch hook-ran-marker",
    );
    let app = format!("file://{}", repo.display());
    let app_id = app_id_for_file_url(&app);

    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    fs::write(
        home.path().join(".bmx/config.toml"),
        r#"default_source = "https://github.com"
hooks_enabled = true
integrity_check = false
"#,
    )
    .expect("config");

    bmx(home.path()).args(["install", &app]).assert().success();

    let marker = home
        .path()
        .join(".bmx/apps")
        .join(&app_id)
        .join("repo/hook-ran-marker");
    assert!(
        marker.exists(),
        "post_install hook should create {}",
        marker.display()
    );
}

#[test]
fn registry_alias_resolves_via_config() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");

    let source_repo = init_make_repo(repos.path(), "reg-src", "regtool", "REGISTRY_OK");
    let bare = repos.path().join("regtool.git");
    let st = Command::new("git")
        .args([
            "clone",
            "--bare",
            source_repo.to_string_lossy().as_ref(),
            bare.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("git");
    assert!(st.success());

    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    let base_url = format!("file://{}", repos.path().display());
    fs::write(
        home.path().join(".bmx/config.toml"),
        format!(
            r#"default_source = "https://github.com"
integrity_check = false

[registries]
loc = "{base_url}"
"#
        ),
    )
    .expect("config");

    bmx(home.path())
        .args(["install", "loc:regtool"])
        .assert()
        .success()
        .stdout(predicate::str::contains("installed"));

    bmx(home.path())
        .args(["exec", "loc:regtool"])
        .assert()
        .success()
        .stdout(predicate::str::contains("REGISTRY_OK"));
}

#[test]
fn integrity_check_fails_when_repo_head_diverges() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "int-tool", "int-tool", "INT_OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path()).args(["install", &app]).assert().success();

    // Overwrite (do not append): serde may emit `[registries]` and later keys would nest wrongly.
    fs::write(
        home.path().join(".bmx/config.toml"),
        r#"default_source = "https://github.com"
build_isolation = "off"
checkout_backend = "git"
integrity_check = true
checkout_profiles = []
"#,
    )
    .expect("write config");

    bmx(home.path()).args(["exec", &app]).assert().success();

    let app_id = app_id_for_file_url(&app);
    let cached = home.path().join(".bmx/apps").join(&app_id).join("repo");
    run_git(&cached, &["commit", "--allow-empty", "-m", "drift"]);

    bmx(home.path())
        .args(["exec", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains("install cache integrity"));
}
