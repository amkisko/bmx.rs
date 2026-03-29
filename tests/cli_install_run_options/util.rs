use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::*;

pub(super) fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=BMX Test",
        ])
        .args(args)
        .status()
        .expect("git command should run");
    assert!(status.success(), "git {:?} should succeed", args);
}

pub(super) fn init_make_repo(
    root: &Path,
    repo_name: &str,
    bin_name: &str,
    msg: &str,
) -> std::path::PathBuf {
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

pub(super) fn init_repo_with_post_install(
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

pub(super) fn app_id_for_file_url(app: &str) -> String {
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

pub(super) fn bmx(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("bmx").expect("binary should build");
    cmd.env("HOME", home);
    cmd.env("BMX_TRUST_ASSUME_YES", "1");
    cmd
}

pub(super) fn bmx_no_trust_assume(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("bmx").expect("binary should build");
    cmd.env("HOME", home);
    cmd
}
