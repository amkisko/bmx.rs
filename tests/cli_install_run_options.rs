//! Integration tests for install/run **controls**: ephemeral cache (`--rm`), project pin (`.bmx/pin`),
//! verbose resolution (`-v`), `integrity_check`, `[registries]` aliases, `bmx.toml` hooks, and `shim` subcommands.
//! Complements `cli.rs` (core workflows) and `version_pinning_cli.rs` (ref pinning).
use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .status()
        .expect("git command should run");
    assert!(status.success(), "git {:?} should succeed", args);
}

fn init_make_repo(root: &Path, repo_name: &str, bin_name: &str, msg: &str) -> std::path::PathBuf {
    let repo = root.join(repo_name);
    fs::create_dir_all(&repo).expect("create repo dir");
    let makefile = format!(
        "all:\n\tmkdir -p bin\n\tprintf '#!/bin/sh\\necho {}\\n' > bin/{}\n\tchmod +x bin/{}\n",
        msg, bin_name, bin_name
    );
    fs::write(repo.join("Makefile"), makefile).expect("write Makefile");
    fs::write(
        repo.join("bmx.toml"),
        format!("[bmx]\nrun = \"bin/{}\"\n", bin_name),
    )
    .expect("write bmx.toml");
    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "test@example.com"]);
    run_git(&repo, &["config", "user.name", "BMX Test"]);
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "init"]);
    repo
}

fn init_repo_with_post_install(
    root: &Path,
    repo_name: &str,
    bin_name: &str,
    msg: &str,
    post_install: &str,
) -> std::path::PathBuf {
    let repo = root.join(repo_name);
    fs::create_dir_all(&repo).expect("create repo dir");
    let makefile = format!(
        "all:\n\tmkdir -p bin\n\tprintf '#!/bin/sh\\necho {}\\n' > bin/{}\n\tchmod +x bin/{}\n",
        msg, bin_name, bin_name
    );
    fs::write(repo.join("Makefile"), makefile).expect("write Makefile");
    let manifest = format!(
        "[bmx]\nrun = \"bin/{bin_name}\"\n\n[bmx.hooks]\npost_install = \"{post_install}\"\n"
    );
    fs::write(repo.join("bmx.toml"), manifest).expect("write bmx.toml");
    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "test@example.com"]);
    run_git(&repo, &["config", "user.name", "BMX Test"]);
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "init"]);
    repo
}

fn app_id_for_file_url(app: &str) -> String {
    let base = app.trim_start_matches("file://");
    base.trim_end_matches(".git")
        .rsplit('/')
        .next()
        .unwrap_or(base)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn bmx(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("bmx").expect("binary should build");
    cmd.env("HOME", home);
    cmd
}

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
