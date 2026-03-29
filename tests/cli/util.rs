use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::prelude::*;

pub(super) fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .status()
        .expect("git command should run");
    assert!(status.success(), "git {:?} should succeed", args);
}

pub(super) fn git_stdout(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("git command should run");
    assert!(output.status.success(), "git {:?} should succeed", args);
    String::from_utf8(output.stdout)
        .expect("utf8 git output")
        .trim()
        .to_string()
}

pub(super) fn init_make_repo(root: &Path, repo_name: &str, bin_name: &str, msg: &str) -> PathBuf {
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

pub(super) fn init_arg_echo_repo(root: &Path, repo_name: &str, bin_name: &str) -> PathBuf {
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

pub(super) fn init_subdir_make_repo(
    root: &Path,
    repo_name: &str,
    bin_name: &str,
    msg: &str,
) -> PathBuf {
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

pub(super) fn app_id_for_input(app: &str) -> String {
    let base = if app.starts_with("git@") && app.matches('@').count() == 1 {
        app
    } else if let Some(idx) = app.rfind('@') {
        let right = &app[idx + 1..];
        if !(right.is_empty() || app.contains("://") && right.contains('/')) {
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

pub(super) fn bmx(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("bmx").expect("binary should build");
    cmd.env("HOME", home);
    cmd.env("BMX_TRUST_ASSUME_YES", "1");
    cmd
}
