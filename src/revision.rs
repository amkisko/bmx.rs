use std::path::Path;

use anyhow::{Context, Result, bail};
use semver::{Version, VersionReq};

use crate::git_cmd::{git_output, git_run};
use crate::process::has_tool;
use crate::runtime::subprocess_output_visible;

pub(crate) fn checkout_requested_ref(
    repo_dir: &Path,
    requested_ref: Option<&str>,
    cli_verbose: bool,
) -> Result<Option<String>> {
    if !has_tool("git") {
        bail!("revision checkout requires `git` on PATH");
    }

    let target = match requested_ref {
        Some(request) => resolve_target_spec(repo_dir, request)?,
        None => current_head_ref_or_commit(repo_dir)?,
    };

    let quiet_git = !subprocess_output_visible(cli_verbose);
    checkout_target(repo_dir, &target, quiet_git).map(Some)
}

fn current_head_ref_or_commit(repo_dir: &Path) -> Result<String> {
    match git_output(repo_dir, &["symbolic-ref", "-q", "HEAD"], &[]) {
        Ok(name) => Ok(name),
        Err(_) => git_output(repo_dir, &["rev-parse", "HEAD"], &[]),
    }
}

fn resolve_target_spec(repo_dir: &Path, requested: &str) -> Result<String> {
    if let Some(tag) = resolve_semver_tag(repo_dir, requested)? {
        return Ok(format!("refs/tags/{tag}"));
    }
    Ok(requested.to_string())
}

fn checkout_target(repo_dir: &Path, target_spec: &str, quiet_git: bool) -> Result<String> {
    git_run(
        repo_dir,
        &["clean", "-fd"],
        &[],
        quiet_git,
    )
    .with_context(|| format!("failed to clean before checkout of `{target_spec}`"))?;
    git_run(
        repo_dir,
        &["checkout", "--force", target_spec],
        &[],
        quiet_git,
    )
    .with_context(|| {
        format!("failed to resolve git reference `{target_spec}` or checkout failed")
    })?;
    git_output(repo_dir, &["rev-parse", "HEAD"], &[]).context("failed to read HEAD after checkout")
}

fn resolve_semver_tag(repo_dir: &Path, requested: &str) -> Result<Option<String>> {
    let req = parse_version_req(requested);
    if req.is_none() {
        return Ok(None);
    }
    let req = req.expect("checked above");

    let tags_out = git_output(repo_dir, &["tag", "-l"], &[]).context("failed to list repository tags")?;
    let mut best: Option<(Version, String)> = None;
    for name in tags_out.lines() {
        let Some(version) = parse_version(name) else {
            continue;
        };
        if req.matches(&version) {
            if best.as_ref().is_none_or(|(current, _)| version > *current) {
                best = Some((version, name.to_string()));
            }
        }
    }

    if let Some((_, tag)) = best {
        return Ok(Some(tag));
    }
    bail!("no git tag matches semver requirement `{requested}`")
}

fn parse_version_req(input: &str) -> Option<VersionReq> {
    if let Some(ver) = parse_version(input) {
        return VersionReq::parse(&format!("={ver}")).ok();
    }
    VersionReq::parse(input).ok()
}

fn parse_version(input: &str) -> Option<Version> {
    let stripped = input.strip_prefix('v').unwrap_or(input);
    Version::parse(stripped).ok()
}
