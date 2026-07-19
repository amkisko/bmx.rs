use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::git_cmd::{git_output, git_run};

use super::policy::{
    best_rule, load_policy, looks_like_ssh_pubkey, normalize_key, source_scope_id,
};

pub(crate) fn trust_allowed_signers_path(home: &Path, source_url: &str) -> PathBuf {
    home.join("trust")
        .join("allowed_signers")
        .join(format!("{}.signers", source_scope_id(source_url)))
}

/// OpenSSH allowed_signers lines need full public keys, not fingerprints.
fn ssh_pubkey_line(key: &str) -> Option<String> {
    let trimmed = key.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || !looks_like_ssh_pubkey(trimmed) {
        return None;
    }
    if trimmed.split_whitespace().count() >= 2 && !trimmed.starts_with('*') {
        Some(format!("* namespaces=\"git\" {trimmed}"))
    } else {
        Some(trimmed.to_string())
    }
}

/// Materialize per-source allowedSignersFile from trust policy public-key entries.
pub(crate) fn sync_allowed_signers_from_policy(home: &Path, source_url: &str) -> Result<PathBuf> {
    let path = trust_allowed_signers_path(home, source_url);
    let dir = home.join("trust").join("allowed_signers");
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let mut lines: Vec<String> = Vec::new();
    lines.push(
        "# Managed by bmx from trust.toml allowed_signing_keys (SSH public keys only).".into(),
    );
    if let Some(policy) = load_policy(home)? {
        let rule = best_rule(&policy, source_url);
        for key in &rule.allowed_signing_keys {
            if let Some(line) = ssh_pubkey_line(key) {
                lines.push(line);
            } else {
                lines.push(format!(
                    "# fingerprint/id (not usable as SSH allowedSigners entry): {}",
                    normalize_key(key)
                ));
            }
        }
    }
    lines.push(String::new());
    std::fs::write(&path, lines.join("\n"))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn trust_git_env(home: &Path, source_url: &str) -> Result<Vec<(String, String)>> {
    let path = sync_allowed_signers_from_policy(home, source_url)?;
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

#[cfg(test)]
mod ssh_pubkey_tests {
    use super::ssh_pubkey_line;

    #[test]
    fn ssh_pubkey_line_accepts_public_keys_only() {
        assert!(ssh_pubkey_line("SHA256:ABCDEF").is_none());
        assert!(ssh_pubkey_line("ABCD1234EF567890").is_none());
        let line = ssh_pubkey_line("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExample comment").unwrap();
        assert!(line.contains("namespaces=\"git\""));
        assert!(line.contains("ssh-ed25519"));
    }
}
