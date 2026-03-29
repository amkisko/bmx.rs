use std::fs;
use std::path::{Path, PathBuf};
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

fn init_make_repo(root: &Path, repo_name: &str, bin_name: &str, msg: &str) -> PathBuf {
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

fn init_arg_echo_repo(root: &Path, repo_name: &str, bin_name: &str) -> PathBuf {
    let repo = root.join(repo_name);
    fs::create_dir_all(&repo).expect("create repo dir");

    let makefile = format!(
        "all:\n\tmkdir -p bin\n\tprintf '#!/bin/sh\\necho \"$$@\"\\n' > bin/{}\n\tchmod +x bin/{}\n",
        bin_name, bin_name
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

fn init_subdir_make_repo(root: &Path, repo_name: &str, bin_name: &str, msg: &str) -> PathBuf {
    let repo = root.join(repo_name);
    let subdir = repo.join("app");
    fs::create_dir_all(&subdir).expect("create repo dir");

    let makefile = format!(
        "all:\n\tmkdir -p bin\n\tprintf '#!/bin/sh\\necho {}\\n' > bin/{}\n\tchmod +x bin/{}\n",
        msg, bin_name, bin_name
    );

    fs::write(subdir.join("Makefile"), makefile).expect("write Makefile");
    fs::write(
        repo.join("bmx.toml"),
        format!("[bmx]\nworkdir = \"app\"\nrun = \"bin/{}\"\n", bin_name),
    )
    .expect("write bmx.toml");

    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "test@example.com"]);
    run_git(&repo, &["config", "user.name", "BMX Test"]);
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "init"]);

    repo
}

fn app_id_for_input(app: &str) -> String {
    let base = if app.starts_with("git@") && app.matches('@').count() == 1 {
        app
    } else if let Some(idx) = app.rfind('@') {
        let right = &app[idx + 1..];
        if !right.is_empty() && !(app.contains("://") && right.contains('/')) {
            &app[..idx]
        } else {
            app
        }
    } else {
        app
    };

    let raw = base
        .trim_end_matches(".git")
        .rsplit('/')
        .next()
        .unwrap_or(base)
        .rsplit(':')
        .next()
        .unwrap_or(base);

    raw.chars()
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
fn cli_without_args_fails_with_helpful_message() {
    let home = TempDir::new().expect("home tempdir");
    bmx(home.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("provide a command"));
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
        .args(["uninstall", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));

    assert!(
        !home.path().join(".bmx/apps").join(app_id).exists(),
        "app directory should be removed after uninstall"
    );
}

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

#[test]
fn debug_mode_shows_command_logs() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");

    let repo = init_make_repo(repos.path(), "debug-tool", "debug-tool", "DEBUG_TOOL_OK");
    let app = format!("file://{}", repo.display());

    let mut cmd = bmx(home.path());
    cmd.env("BMX_DEBUG", "1");
    cmd.args(["install", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("installed"))
        .stderr(predicate::str::contains(
            "[bmx][debug] run_checked: make -j",
        ));
}

#[test]
fn install_history_and_undo_removes_fresh_install() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");
    let repo = init_make_repo(repos.path(), "undo-tool", "undo-tool", "UNDO_OK");
    let app = format!("file://{}", repo.display());
    let app_id = app_id_for_input(&app);

    bmx(home.path())
        .args(["install", &app])
        .assert()
        .success();
    assert!(
        home.path().join(".bmx/apps").join(&app_id).exists(),
        "app dir should exist after install"
    );

    bmx(home.path())
        .args(["history", "-n", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("install"));

    bmx(home.path())
        .args(["undo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("undone"));

    assert!(
        !home.path().join(".bmx/apps").join(&app_id).exists(),
        "undo should remove a fresh install"
    );
}

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

    bmx(home.path())
        .args(["install", &app1])
        .assert()
        .success();
    bmx(home.path())
        .args(["install", &app2])
        .assert()
        .success();

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

    bmx(home.path())
        .args(["install", &app1])
        .assert()
        .success();

    bmx(home.path())
        .args(["install", &app2])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--as"));
}
