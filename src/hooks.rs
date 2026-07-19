use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::config::load_config;
use crate::io::read_toml;
use crate::types::BMXManifest;

pub(crate) fn run_pre_run_hook(home: &Path, repo_root: &Path) -> Result<()> {
    if !hooks_enabled(home)? {
        return Ok(());
    }
    let path = repo_root.join("bmx.toml");
    if !path.exists() {
        return Ok(());
    }
    let manifest: BMXManifest = read_toml(&path)?;
    if let Some(cmd) = manifest.bmx.and_then(|b| b.hooks).and_then(|h| h.pre_run) {
        run_shell_hook(repo_root, &cmd, "pre_run")?;
    }
    Ok(())
}

pub(crate) fn run_post_install_hook(home: &Path, repo_root: &Path) -> Result<()> {
    if !hooks_enabled(home)? {
        return Ok(());
    }
    let path = repo_root.join("bmx.toml");
    if !path.exists() {
        return Ok(());
    }
    let manifest: BMXManifest = read_toml(&path)?;
    if let Some(cmd) = manifest
        .bmx
        .and_then(|b| b.hooks)
        .and_then(|h| h.post_install)
    {
        run_shell_hook(repo_root, &cmd, "post_install")?;
    }
    Ok(())
}

fn hooks_enabled(home: &Path) -> Result<bool> {
    Ok(load_config(home)?.hooks_enabled)
}

pub(crate) fn run_shell_hook(repo_root: &Path, command: &str, label: &str) -> Result<()> {
    let cmd = command.trim();
    if cmd.is_empty() {
        return Ok(());
    }
    let status = Command::new("sh")
        .current_dir(repo_root)
        .args(["-lc", cmd])
        .status()
        .with_context(|| format!("failed to spawn {label} hook"))?;
    if !status.success() {
        anyhow::bail!("{label} hook failed with status {status}: {cmd}");
    }
    Ok(())
}
