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

fn git_stdout(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("git command should run");
    assert!(output.status.success(), "git {:?} should succeed", args);
    String::from_utf8(output.stdout)
        .expect("utf8 output")
        .trim()
        .to_string()
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

fn bmx(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("bmx").expect("binary should build");
    cmd.env("HOME", home);
    cmd
}

#[test]
fn install_and_exec_support_semver_and_commit_sha_pinning() {
    let home = TempDir::new().expect("home tempdir");
    let repos = TempDir::new().expect("repos tempdir");

    let repo = init_make_repo(repos.path(), "pin-tool", "pin-tool", "V1");
    run_git(&repo, &["tag", "v1.0.0"]);

    let makefile_v2 = "all:\n\tmkdir -p bin\n\tprintf '#!/bin/sh\\necho V2\\n' > bin/pin-tool\n\tchmod +x bin/pin-tool\n";
    fs::write(repo.join("Makefile"), makefile_v2).expect("write Makefile v2");
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "v2"]);
    run_git(&repo, &["tag", "v1.1.0"]);
    let v100_sha = git_stdout(&repo, &["rev-list", "-n", "1", "v1.0.0"]);

    let app = format!("file://{}", repo.display());
    bmx(home.path())
        .args(["exec", &format!("{app}@^1.0")])
        .assert()
        .success()
        .stdout(predicate::str::contains("V2"));

    bmx(home.path())
        .args(["exec", &format!("{app}@{v100_sha}")])
        .assert()
        .success()
        .stdout(predicate::str::contains("V1"));

    let install_meta = home.path().join(".bmx/apps/pin-tool/install.toml");
    let content = fs::read_to_string(install_meta).expect("metadata");
    assert!(content.contains("requested_ref"));
    assert!(content.contains("resolved_commit"));
}
