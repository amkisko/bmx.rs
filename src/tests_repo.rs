use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

use crate::repo::sync_repo;
use crate::revision::checkout_requested_ref;
use crate::types::Config;

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .status()
        .expect("git command");
    assert!(status.success(), "git {:?} failed", args);
}

fn git_stdout(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("git command");
    assert!(output.status.success(), "git {:?} failed", args);
    String::from_utf8(output.stdout)
        .expect("utf8 git output")
        .trim()
        .to_string()
}

fn init_repo(path: &Path, content: &str) {
    fs::create_dir_all(path).expect("mkdir");
    fs::write(path.join("README"), content).expect("write readme");
    run_git(path, &["init"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "BMX Test"]);
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "init"]);
    run_git(path, &["branch", "-M", "main"]);
}

#[test]
fn sync_repo_clones_then_fast_forwards() {
    let temp = TempDir::new().expect("tmp");
    let src = temp.path().join("source");
    init_repo(&src, "v1\n");

    let bare = temp.path().join("origin.git");
    let status = Command::new("git")
        .args([
            "clone",
            "--bare",
            src.to_string_lossy().as_ref(),
            bare.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("clone --bare");
    assert!(status.success());

    let target = temp.path().join("cache/repo");
    fs::create_dir_all(&target).expect("mkdir repo");
    let url = bare.to_string_lossy().to_string();

    sync_repo(&url, &target, &Config::default(), false).expect("first clone");
    assert!(target.join("README").exists());

    fs::write(src.join("README"), "v2\n").expect("update source");
    run_git(&src, &["add", "."]);
    run_git(&src, &["commit", "-m", "update"]);
    run_git(
        &src,
        &["remote", "add", "origin", bare.to_string_lossy().as_ref()],
    );
    run_git(&src, &["push", "origin", "HEAD:main"]);

    sync_repo(&url, &target, &Config::default(), false).expect("fast-forward");
    let readme = fs::read_to_string(target.join("README")).expect("read clone");
    assert!(readme.contains("v2"));
}

#[test]
fn sync_repo_rejects_non_git_non_empty_dir() {
    let temp = TempDir::new().expect("tmp");
    let non_git = temp.path().join("non-git");
    fs::create_dir_all(&non_git).expect("mkdir");
    fs::write(non_git.join("x"), "y").expect("write");

    let err = sync_repo(
        "https://example.invalid/repo.git",
        &non_git,
        &Config::default(),
        false,
    )
    .expect_err("must fail");
    assert!(err.to_string().contains("not empty and not a git repo"));
}

#[test]
fn checkout_requested_ref_supports_semver_and_sha() {
    let temp = TempDir::new().expect("tmp");
    let src = temp.path().join("source");
    init_repo(&src, "v1\n");
    run_git(&src, &["tag", "v1.0.0"]);

    fs::write(src.join("README"), "v2\n").expect("update source");
    run_git(&src, &["add", "."]);
    run_git(&src, &["commit", "-m", "v2"]);
    run_git(&src, &["tag", "v1.1.0"]);

    let bare = temp.path().join("origin.git");
    let status = Command::new("git")
        .args([
            "clone",
            "--bare",
            src.to_string_lossy().as_ref(),
            bare.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("clone --bare");
    assert!(status.success());

    let target = temp.path().join("cache/repo");
    fs::create_dir_all(&target).expect("mkdir repo");
    sync_repo(&bare.to_string_lossy(), &target, &Config::default(), false).expect("clone");

    checkout_requested_ref(&target, Some("^1.0"), false).expect("checkout semver");
    let semver_head = git_stdout(&target, &["rev-parse", "HEAD"]);
    let v110 = git_stdout(&target, &["rev-list", "-n", "1", "v1.1.0"]);
    assert_eq!(semver_head, v110);

    let v100 = git_stdout(&target, &["rev-list", "-n", "1", "v1.0.0"]);
    checkout_requested_ref(&target, Some(&v100), false).expect("checkout sha");
    let sha_head = git_stdout(&target, &["rev-parse", "HEAD"]);
    assert_eq!(sha_head, v100);
}

#[test]
fn checkout_requested_ref_fails_for_unmatched_semver() {
    let temp = TempDir::new().expect("tmp");
    let src = temp.path().join("source");
    init_repo(&src, "v1\n");
    run_git(&src, &["tag", "v1.0.0"]);

    let bare = temp.path().join("origin.git");
    let status = Command::new("git")
        .args([
            "clone",
            "--bare",
            src.to_string_lossy().as_ref(),
            bare.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("clone --bare");
    assert!(status.success());

    let target = temp.path().join("cache/repo");
    fs::create_dir_all(&target).expect("mkdir repo");
    sync_repo(&bare.to_string_lossy(), &target, &Config::default(), false).expect("clone");

    let err = checkout_requested_ref(&target, Some("^2.0"), false).expect_err("must fail");
    assert!(
        err.to_string()
            .contains("no git tag matches semver requirement")
    );
}
