use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::git_cmd::{git_output, git_run};

use super::policy::{normalize_key, source_scope_id};

pub(crate) fn trust_allowed_signers_path(home: &Path, source_url: &str) -> PathBuf {
    home.join("trust")
        .join("allowed_signers")
        .join(format!("{}.signers", source_scope_id(source_url)))
}

pub(crate) fn trust_git_env(home: &Path, source_url: &str) -> Result<Vec<(String, String)>> {
    let path = trust_allowed_signers_path(home, source_url);
    let dir = home.join("trust").join("allowed_signers");
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    if !path.exists() {
        std::fs::write(&path, "")
            .with_context(|| format!("failed to create {}", path.display()))?;
    }
    Ok(vec![
        ("GIT_CONFIG_COUNT".to_string(), "1".to_string()),
        (
            "GIT_CONFIG_KEY_0".to_string(),
            "gpg.ssh.allowedSignersFile".to_string(),
        ),
        (
            "GIT_CONFIG_VALUE_0".to_string(),
            path.to_string_lossy().to_string(),
        ),
    ])
}

pub(crate) fn trust_git_output(
    home: &Path,
    source_url: &str,
    repo_dir: &Path,
    args: &[&str],
) -> Result<String> {
    let env = trust_git_env(home, source_url)?;
    git_output(repo_dir, args, &env)
}

pub(crate) fn trust_git_run(
    home: &Path,
    source_url: &str,
    repo_dir: &Path,
    args: &[&str],
    quiet: bool,
) -> Result<()> {
    let env = trust_git_env(home, source_url)?;
    git_run(repo_dir, args, &env, quiet)
}

pub(crate) fn commit_signing_key(home: &Path, source_url: &str, repo_dir: &Path) -> Option<String> {
    trust_git_output(home, source_url, repo_dir, &["log", "-1", "--format=%GK"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) fn commit_signing_fingerprint(
    home: &Path,
    source_url: &str,
    repo_dir: &Path,
) -> Option<String> {
    trust_git_output(home, source_url, repo_dir, &["log", "-1", "--format=%GF"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[allow(clippy::collapsible_if)]
pub(crate) fn repo_signing_keys(home: &Path, source_url: &str, repo_dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(v) = commit_signing_key(home, source_url, repo_dir).map(|s| normalize_key(&s)) {
        out.push(v);
    }
    if let Some(v) =
        commit_signing_fingerprint(home, source_url, repo_dir).map(|s| normalize_key(&s))
    {
        if !out.iter().any(|x| x == &v) {
            out.push(v);
        }
    }
    out
}
