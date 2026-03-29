use anyhow::{Result, bail};

use crate::git_cmd::git_output;
use crate::layout::AppLayout;
use crate::process::has_tool;
use crate::types::InstallMetadata;

/// When `resolved_commit` is set, ensure the repo worktree HEAD matches (detect tampering or stale state).
pub(crate) fn verify_repo_matches_metadata(
    layout: &AppLayout,
    metadata: &InstallMetadata,
) -> Result<()> {
    let Some(expected) = metadata.resolved_commit.as_deref() else {
        return Ok(());
    };
    if !layout.repo_dir.join(".git").exists() {
        return Ok(());
    }
    if !has_tool("git") {
        return Ok(());
    }

    let actual = git_output(&layout.repo_dir, &["rev-parse", "HEAD"], &[])?;
    if actual != expected {
        bail!(
            "install cache integrity: repository HEAD `{}` does not match install.toml resolved_commit `{}`; run `bmx reinstall {}` or `bmx install {}`",
            actual,
            expected,
            metadata.app,
            metadata.app
        );
    }
    Ok(())
}
