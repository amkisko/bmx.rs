use std::path::Path;
use std::process::Command;

use crate::git_cmd::git_output;
use crate::types::CheckoutBackend;
use tempfile::tempdir;

use crate::repo::CheckoutPlan;

use super::{
    custom_clone, custom_sync, git_cli_fetch_and_ff, github_repo_slug, render_custom, run_status,
    shell_quote,
};

fn run_git(repo: &Path, args: &[&str]) {
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

#[test]
fn github_repo_slug_handles_common_forms() {
    assert_eq!(
        github_repo_slug("git@github.com:acme/tool.git").as_deref(),
        Some("acme/tool")
    );
    assert_eq!(
        github_repo_slug("https://github.com/acme/tool").as_deref(),
        Some("acme/tool")
    );
    assert_eq!(github_repo_slug("https://gitlab.com/acme/tool"), None);
}

#[test]
fn shell_quote_and_render_custom_escape_inputs() {
    assert_eq!(shell_quote(""), "''");
    assert_eq!(shell_quote("a'b"), "'a'\"'\"'b'");
    let rendered = render_custom(
        "clone {source_url} {repo_dir}",
        "https://github.com/acme/tool.git",
        Path::new("/tmp/a b"),
    );
    assert!(rendered.contains("'https://github.com/acme/tool.git'"));
    assert!(rendered.contains("'/tmp/a b'"));
}

#[test]
fn run_status_success_and_failure() {
    run_status(None, "sh", &["-lc".into(), "true".into()], &[]).unwrap();
    let err = run_status(None, "sh", &["-lc".into(), "false".into()], &[]).unwrap_err();
    assert!(err.to_string().contains("command failed"));
}

#[test]
fn custom_clone_and_sync_execute_template_commands() {
    let td = tempdir().unwrap();
    let repo_dir = td.path().join("repo");
    std::fs::create_dir_all(&repo_dir).unwrap();
    let marker = td.path().join("marker.txt");
    let plan = CheckoutPlan {
        backend: CheckoutBackend::Custom,
        env: vec![],
        custom_clone: Some(format!(
            "printf cloned > {}",
            shell_quote(&marker.to_string_lossy())
        )),
        custom_sync: Some(format!(
            "printf synced > {}",
            shell_quote(&marker.to_string_lossy())
        )),
    };

    custom_clone("https://example.com/repo.git", &repo_dir, &plan).unwrap();
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "cloned");
    custom_sync(&repo_dir, &plan).unwrap();
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "synced");
}

#[test]
fn git_cli_fetch_and_ff_works_for_simple_origin_main() {
    let td = tempdir().unwrap();
    let src = td.path().join("src");
    let origin = td.path().join("origin.git");
    let clone = td.path().join("clone");
    std::fs::create_dir_all(&src).unwrap();
    run_git(&src, &["init"]);
    std::fs::write(src.join("README.md"), "v1").unwrap();
    run_git(&src, &["add", "."]);
    run_git(&src, &["commit", "-m", "init"]);
    run_git(
        &src,
        &[
            "clone",
            "--bare",
            src.to_str().unwrap(),
            origin.to_str().unwrap(),
        ],
    );
    run_git(&src, &["remote", "add", "origin", origin.to_str().unwrap()]);
    run_git(&src, &["push", "-u", "origin", "HEAD"]);
    run_git(
        td.path(),
        &["clone", origin.to_str().unwrap(), clone.to_str().unwrap()],
    );

    std::fs::write(src.join("README.md"), "v2").unwrap();
    run_git(&src, &["add", "."]);
    run_git(&src, &["commit", "-m", "v2"]);
    run_git(&src, &["push", "origin", "HEAD"]);

    git_cli_fetch_and_ff(&clone, &[], true).unwrap();
    let head = git_output(&clone, &["show", "-s", "--format=%s", "HEAD"], &[]).unwrap();
    assert_eq!(head, "v2");
}
