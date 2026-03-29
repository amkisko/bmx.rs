use std::path::Path;
use std::{io::IsTerminal, io::Write};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::git_cmd::{git_output, git_run};
use crate::io::{read_toml, write_toml};
use crate::runtime::debug_log;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct TrustPolicy {
    #[serde(default)]
    default: TrustRule,
    #[serde(default)]
    rules: Vec<TrustRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TrustRule {
    #[serde(default)]
    match_prefix: Option<String>,
    #[serde(default = "default_allow")]
    allow: bool,
    #[serde(default)]
    require_signed_commit: bool,
    #[serde(default)]
    allowed_signing_keys: Vec<String>,
}

impl Default for TrustRule {
    fn default() -> Self {
        Self {
            match_prefix: None,
            allow: true,
            require_signed_commit: false,
            allowed_signing_keys: Vec::new(),
        }
    }
}

fn default_allow() -> bool {
    true
}

fn trust_policy_path(home: &Path) -> std::path::PathBuf {
    home.join("trust.toml")
}

fn load_policy(home: &Path) -> Result<Option<TrustPolicy>> {
    let path = trust_policy_path(home);
    if !path.exists() {
        return Ok(None);
    }
    let policy = read_toml::<TrustPolicy>(&path)
        .with_context(|| format!("failed to load trust policy {}", path.display()))?;
    Ok(Some(policy))
}

fn load_policy_or_default(home: &Path) -> Result<TrustPolicy> {
    Ok(load_policy(home)?.unwrap_or_default())
}

fn save_policy(home: &Path, policy: &TrustPolicy) -> Result<()> {
    let path = trust_policy_path(home);
    write_toml(&path, policy).with_context(|| format!("failed to write {}", path.display()))
}

fn best_rule<'a>(policy: &'a TrustPolicy, source_url: &str) -> &'a TrustRule {
    let mut best: Option<&TrustRule> = None;
    let mut best_len = 0usize;
    for rule in &policy.rules {
        let Some(prefix) = rule.match_prefix.as_deref() else {
            continue;
        };
        if source_url.starts_with(prefix) && prefix.len() > best_len {
            best = Some(rule);
            best_len = prefix.len();
        }
    }
    best.unwrap_or(&policy.default)
}

fn normalize_key(s: &str) -> String {
    s.trim().to_ascii_uppercase()
}

fn commit_signing_key(repo_dir: &Path) -> Option<String> {
    git_output(repo_dir, &["log", "-1", "--format=%GK"], &[])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn commit_signing_fingerprint(repo_dir: &Path) -> Option<String> {
    git_output(repo_dir, &["log", "-1", "--format=%GF"], &[])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) fn repo_signing_keys(repo_dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(v) = commit_signing_key(repo_dir).map(|s| normalize_key(&s)) {
        out.push(v);
    }
    if let Some(v) = commit_signing_fingerprint(repo_dir).map(|s| normalize_key(&s))
        && !out.iter().any(|x| x == &v)
    {
        out.push(v);
    }
    out
}

fn enforce_signed_commit(repo_dir: &Path, allowed_signing_keys: &[String]) -> Result<()> {
    if !repo_dir.join(".git").exists() {
        bail!(
            "trust policy requires signed commit, but {} is not a git repository",
            repo_dir.display()
        );
    }

    git_run(repo_dir, &["verify-commit", "HEAD"], &[], true).with_context(|| {
        format!(
            "trust policy requires signed commit, but HEAD failed signature verification in {}",
            repo_dir.display()
        )
    })?;

    if allowed_signing_keys.is_empty() {
        return Ok(());
    }

    let key = commit_signing_key(repo_dir).map(|s| normalize_key(&s));
    let fp = commit_signing_fingerprint(repo_dir).map(|s| normalize_key(&s));
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
        enforce_signed_commit(repo_dir, &rule.allowed_signing_keys)?;
    }
    Ok(())
}

fn mutable_rule_for_match_prefix<'a>(
    policy: &'a mut TrustPolicy,
    match_prefix: &str,
) -> &'a mut TrustRule {
    if let Some(idx) = policy
        .rules
        .iter()
        .position(|r| r.match_prefix.as_deref() == Some(match_prefix))
    {
        return &mut policy.rules[idx];
    }

    policy.rules.push(TrustRule {
        match_prefix: Some(match_prefix.to_string()),
        ..TrustRule::default()
    });
    let idx = policy.rules.len() - 1;
    &mut policy.rules[idx]
}

pub(crate) fn show_policy_toml(home: &Path) -> Result<String> {
    let policy = load_policy_or_default(home)?;
    toml::to_string(&policy).context("failed to serialize trust policy")
}

pub(crate) fn add_allowed_signing_key(
    home: &Path,
    key: &str,
    match_prefix: Option<&str>,
) -> Result<()> {
    let key = normalize_key(key);
    if key.is_empty() {
        bail!("signing key is empty");
    }

    let mut policy = load_policy_or_default(home)?;
    let rule = if let Some(prefix) = match_prefix {
        mutable_rule_for_match_prefix(&mut policy, prefix)
    } else {
        &mut policy.default
    };
    if !rule
        .allowed_signing_keys
        .iter()
        .any(|k| normalize_key(k) == key)
    {
        rule.allowed_signing_keys.push(key);
    }
    save_policy(home, &policy)
}

pub(crate) fn set_require_signed_commit(
    home: &Path,
    match_prefix: &str,
    enabled: bool,
) -> Result<()> {
    if match_prefix.trim().is_empty() {
        bail!("match-prefix is empty");
    }
    let mut policy = load_policy_or_default(home)?;
    let rule = mutable_rule_for_match_prefix(&mut policy, match_prefix);
    rule.require_signed_commit = enabled;
    save_policy(home, &policy)
}

pub(crate) fn set_allow(home: &Path, match_prefix: &str, allow: bool) -> Result<()> {
    if match_prefix.trim().is_empty() {
        bail!("match-prefix is empty");
    }
    let mut policy = load_policy_or_default(home)?;
    let rule = mutable_rule_for_match_prefix(&mut policy, match_prefix);
    rule.allow = allow;
    save_policy(home, &policy)
}

pub(crate) fn import_signing_keys_from_repo(
    home: &Path,
    repo_dir: &Path,
    match_prefix: Option<&str>,
) -> Result<usize> {
    let keys = repo_signing_keys(repo_dir);
    if keys.is_empty() {
        bail!(
            "no commit signing key/fingerprint found for HEAD in {}",
            repo_dir.display()
        );
    }
    let mut policy = load_policy_or_default(home)?;
    let rule = if let Some(prefix) = match_prefix {
        mutable_rule_for_match_prefix(&mut policy, prefix)
    } else {
        &mut policy.default
    };
    for key in &keys {
        if !rule
            .allowed_signing_keys
            .iter()
            .any(|k| normalize_key(k) == *key)
        {
            rule.allowed_signing_keys.push(key.clone());
        }
    }
    save_policy(home, &policy)?;
    Ok(keys.len())
}

fn rule_for_source<'a>(policy: &'a TrustPolicy, source_url: &str) -> &'a TrustRule {
    best_rule(policy, source_url)
}

fn keys_missing_for_source_scope(
    home: &Path,
    source_url: &str,
    keys: &[String],
) -> Result<Vec<String>> {
    let policy = load_policy_or_default(home)?;
    let rule = rule_for_source(&policy, source_url);
    let mut out = Vec::new();
    for key in keys {
        let present = rule
            .allowed_signing_keys
            .iter()
            .any(|existing| normalize_key(existing) == normalize_key(key));
        if !present {
            out.push(key.clone());
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
struct HeadAssessment {
    commit: String,
    short_commit: String,
    author_name: String,
    author_email: String,
    authored_at: String,
    subject: String,
    sig_status: String,
    sig_signer: String,
    sig_key: String,
    sig_fingerprint: String,
}

fn head_assessment(repo_dir: &Path) -> Option<HeadAssessment> {
    let fmt = "%H%n%h%n%an%n%ae%n%aI%n%s%n%G?%n%GS%n%GK%n%GF";
    let raw = git_output(repo_dir, &["log", "-1", &format!("--format={fmt}")], &[]).ok()?;
    let mut lines = raw.lines();
    Some(HeadAssessment {
        commit: lines.next().unwrap_or_default().to_string(),
        short_commit: lines.next().unwrap_or_default().to_string(),
        author_name: lines.next().unwrap_or_default().to_string(),
        author_email: lines.next().unwrap_or_default().to_string(),
        authored_at: lines.next().unwrap_or_default().to_string(),
        subject: lines.next().unwrap_or_default().to_string(),
        sig_status: lines.next().unwrap_or_default().to_string(),
        sig_signer: lines.next().unwrap_or_default().to_string(),
        sig_key: lines.next().unwrap_or_default().to_string(),
        sig_fingerprint: lines.next().unwrap_or_default().to_string(),
    })
}

fn sig_status_human(code: &str) -> &'static str {
    match code {
        "G" => "good signature",
        "U" => "good signature (untrusted key)",
        "B" => "bad signature",
        "N" => "no signature",
        "E" => "signature verification error",
        _ => "unknown signature state",
    }
}

pub(crate) fn prompt_import_signing_keys_for_source(
    home: &Path,
    source_url: &str,
    repo_dir: &Path,
) -> Result<()> {
    let keys = repo_signing_keys(repo_dir);
    if keys.is_empty() {
        eprintln!(
            "[bmx][trust] no signer key/fingerprint found on HEAD (nothing to import): {}",
            repo_dir.display()
        );
        return Ok(());
    }
    let missing = keys_missing_for_source_scope(home, source_url, &keys)?;
    if missing.is_empty() {
        eprintln!("[bmx][trust] signer key already trusted for {source_url}");
        return Ok(());
    }

    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        bail!(
            "--trust requires an interactive terminal to confirm signer key import for {source_url}"
        );
    }

    let assessment = head_assessment(repo_dir);
    eprintln!("[bmx][trust] source: {source_url}");
    eprintln!("[bmx][trust] repo: {}", repo_dir.display());
    if let Some(a) = &assessment {
        eprintln!(
            "[bmx][trust] head: {} ({})",
            a.short_commit,
            if a.commit.is_empty() { "-" } else { &a.commit }
        );
        eprintln!(
            "[bmx][trust] author: {} <{}>",
            a.author_name, a.author_email
        );
        eprintln!("[bmx][trust] date: {}", a.authored_at);
        eprintln!("[bmx][trust] subject: {}", a.subject);
        if !a.sig_status.is_empty() {
            eprintln!(
                "[bmx][trust] signature: {} ({})",
                sig_status_human(&a.sig_status),
                a.sig_status
            );
        }
        if !a.sig_signer.is_empty() {
            eprintln!("[bmx][trust] signer: {}", a.sig_signer);
        }
        if !a.sig_key.is_empty() {
            eprintln!("[bmx][trust] signer key id: {}", a.sig_key);
        }
        if !a.sig_fingerprint.is_empty() {
            eprintln!("[bmx][trust] signer fingerprint: {}", a.sig_fingerprint);
        }
    }
    eprintln!("[bmx][trust] proposed keys:");
    for key in &missing {
        eprintln!("[bmx][trust]   - {key}");
    }

    let mut stderr = std::io::stderr();
    write!(
        stderr,
        "[bmx][trust] add these keys to trust policy for this source? [y/N]: "
    )?;
    stderr.flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let confirmed = matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes");
    if !confirmed {
        eprintln!("[bmx][trust] declined key import for {source_url}");
        return Ok(());
    }

    for key in &missing {
        add_allowed_signing_key(home, key, Some(source_url))?;
    }
    eprintln!(
        "[bmx][trust] imported {} key(s) for {}",
        missing.len(),
        source_url
    );
    Ok(())
}

fn env_truthy(name: &str) -> bool {
    std::env::var(name)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn signer_matches_allowed(rule: &TrustRule, repo_dir: &Path) -> bool {
    if rule.allowed_signing_keys.is_empty() {
        return false;
    }
    let key = commit_signing_key(repo_dir).map(|s| normalize_key(&s));
    let fp = commit_signing_fingerprint(repo_dir).map(|s| normalize_key(&s));
    let allowed: Vec<String> = rule
        .allowed_signing_keys
        .iter()
        .map(|s| normalize_key(s))
        .collect();
    key.as_ref().is_some_and(|k| allowed.iter().any(|a| a == k))
        || fp.as_ref().is_some_and(|f| allowed.iter().any(|a| a == f))
}

/// Ask consent before proceeding with install/update/rebuild when the source is not trusted.
///
/// A source is treated as trusted when either:
/// - HEAD has a verified-good signature (`%G? == G`), or
/// - the signer key/fingerprint matches `allowed_signing_keys` in the effective trust rule.
pub(crate) fn prompt_untrusted_source_consent(
    home: &Path,
    source_url: &str,
    repo_dir: &Path,
) -> Result<()> {
    let policy = load_policy_or_default(home)?;
    let rule = best_rule(&policy, source_url);
    if !rule.allow {
        return Ok(());
    }

    let assessment = head_assessment(repo_dir);
    let sig_status = assessment
        .as_ref()
        .map(|a| a.sig_status.as_str())
        .unwrap_or_default();
    let good_signature = sig_status == "G";
    let trusted_signer = signer_matches_allowed(rule, repo_dir);
    debug_log(&format!(
        "trust consent source={} sig_status={} good_signature={} trusted_signer={} allow={} keys={}",
        source_url,
        sig_status,
        good_signature,
        trusted_signer,
        rule.allow,
        rule.allowed_signing_keys.len()
    ));
    if good_signature || trusted_signer {
        return Ok(());
    }

    if env_truthy("BMX_TRUST_ASSUME_YES") {
        eprintln!(
            "[bmx][trust] auto-consent enabled via BMX_TRUST_ASSUME_YES for untrusted source {source_url}"
        );
        return Ok(());
    }

    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        bail!(
            "untrusted source requires interactive consent (no verified signature/trusted signer): {source_url}"
        );
    }

    eprintln!("[bmx][trust] untrusted source assessment");
    eprintln!("[bmx][trust] source: {source_url}");
    eprintln!("[bmx][trust] repo: {}", repo_dir.display());
    if let Some(a) = &assessment {
        eprintln!(
            "[bmx][trust] head: {} ({})",
            a.short_commit,
            if a.commit.is_empty() { "-" } else { &a.commit }
        );
        eprintln!(
            "[bmx][trust] author: {} <{}>",
            a.author_name, a.author_email
        );
        eprintln!("[bmx][trust] date: {}", a.authored_at);
        eprintln!("[bmx][trust] subject: {}", a.subject);
        if !a.sig_status.is_empty() {
            eprintln!(
                "[bmx][trust] signature: {} ({})",
                sig_status_human(&a.sig_status),
                a.sig_status
            );
        } else {
            eprintln!("[bmx][trust] signature: unknown");
        }
        if !a.sig_signer.is_empty() {
            eprintln!("[bmx][trust] signer: {}", a.sig_signer);
        }
        if !a.sig_key.is_empty() {
            eprintln!("[bmx][trust] signer key id: {}", a.sig_key);
        }
        if !a.sig_fingerprint.is_empty() {
            eprintln!("[bmx][trust] signer fingerprint: {}", a.sig_fingerprint);
        }
    } else {
        eprintln!("[bmx][trust] unable to read HEAD signing metadata");
    }
    eprintln!(
        "[bmx][trust] reason: source has no verified-good signature and signer is not trusted in policy"
    );

    let mut stderr = std::io::stderr();
    write!(
        stderr,
        "[bmx][trust] continue with this untrusted source? [y/N]: "
    )?;
    stderr.flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let confirmed = matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes");
    if !confirmed {
        bail!("installation aborted by user for untrusted source {source_url}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_rule_prefers_longest_prefix() {
        let policy = TrustPolicy {
            default: TrustRule::default(),
            rules: vec![
                TrustRule {
                    match_prefix: Some("https://github.com/".to_string()),
                    allow: true,
                    require_signed_commit: false,
                    allowed_signing_keys: vec![],
                },
                TrustRule {
                    match_prefix: Some("https://github.com/acme/".to_string()),
                    allow: false,
                    require_signed_commit: false,
                    allowed_signing_keys: vec![],
                },
            ],
        };

        let r = best_rule(&policy, "https://github.com/acme/tool.git");
        assert!(!r.allow);
    }

    #[test]
    fn mutable_rule_creates_and_updates_rule() {
        let mut policy = TrustPolicy::default();
        {
            let r = mutable_rule_for_match_prefix(&mut policy, "https://example.com/");
            r.allow = false;
        }
        assert_eq!(policy.rules.len(), 1);
        assert_eq!(
            policy.rules[0].match_prefix.as_deref(),
            Some("https://example.com/")
        );
        assert!(!policy.rules[0].allow);
    }
}
