use std::path::Path;

use anyhow::{Context, Result, bail};

use super::git_env::{commit_signing_fingerprint, commit_signing_key, trust_git_run};
use super::policy::{TrustRule, best_rule, load_policy, normalize_key};

fn enforce_signed_commit(
    home: &Path,
    source_url: &str,
    repo_dir: &Path,
    allowed_signing_keys: &[String],
) -> Result<()> {
    if !repo_dir.join(".git").exists() {
        bail!(
            "trust policy requires signed commit, but {} is not a git repository",
            repo_dir.display()
        );
    }

    trust_git_run(home, source_url, repo_dir, &["verify-commit", "HEAD"], true).with_context(
        || {
            format!(
                "trust policy requires signed commit, but HEAD failed signature verification in {}",
                repo_dir.display()
            )
        },
    )?;

    if allowed_signing_keys.is_empty() {
        return Ok(());
    }

    let key = commit_signing_key(home, source_url, repo_dir).map(|s| normalize_key(&s));
    let fp = commit_signing_fingerprint(home, source_url, repo_dir).map(|s| normalize_key(&s));
    let allowed: Vec<String> = allowed_signing_keys
        .iter()
        .map(|s| normalize_key(s))
        .collect();
    let key_ok = key.as_ref().is_some_and(|k| allowed.iter().any(|a| a == k));
    let fp_ok = fp.as_ref().is_some_and(|f| allowed.iter().any(|a| a == f));
    if key_ok || fp_ok {
        return Ok(());
    }

    bail!(
        "trust policy signer mismatch: HEAD signing key/fingerprint is not in allowed_signing_keys"
    );
}

/// Enforce optional source trust policy from `~/.bmx/trust.toml`.
///
/// Policy is fail-closed when present and malformed, but no-op when absent.
pub(crate) fn enforce_source_trust(home: &Path, source_url: &str, repo_dir: &Path) -> Result<()> {
    let Some(policy) = load_policy(home)? else {
        return Ok(());
    };
    let rule = best_rule(&policy, source_url);
    if !rule.allow {
        bail!("source is blocked by trust policy: {source_url}");
    }
    if rule.require_signed_commit {
        enforce_signed_commit(home, source_url, repo_dir, &rule.allowed_signing_keys)?;
    }
    Ok(())
}

pub(crate) fn signer_matches_allowed(
    home: &Path,
    source_url: &str,
    rule: &TrustRule,
    repo_dir: &Path,
) -> bool {
    if rule.allowed_signing_keys.is_empty() {
        return false;
    }
    let key = commit_signing_key(home, source_url, repo_dir).map(|s| normalize_key(&s));
    let fp = commit_signing_fingerprint(home, source_url, repo_dir).map(|s| normalize_key(&s));
    let allowed: Vec<String> = rule
        .allowed_signing_keys
        .iter()
        .map(|s| normalize_key(s))
        .collect();
    key.as_ref().is_some_and(|k| allowed.iter().any(|a| a == k))
        || fp.as_ref().is_some_and(|f| allowed.iter().any(|a| a == f))
}

pub(crate) fn env_truthy(name: &str) -> bool {
    std::env::var(name)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}
