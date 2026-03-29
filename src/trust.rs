use std::path::Path;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use std::{fs, path::PathBuf};
use std::{io::IsTerminal, io::Write};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::git_cmd::{git_output, git_run};
use crate::io::read_toml;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&path, render_policy_toml(policy))
        .with_context(|| format!("failed to write {}", path.display()))
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

fn source_scope_id(source_url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    source_url.hash(&mut hasher);
    let hash = format!("{:016x}", hasher.finish());
    let mut slug: String = source_url
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    if slug.len() > 64 {
        slug.truncate(64);
    }
    format!("{slug}-{hash}")
}

fn trust_allowed_signers_path(home: &Path, source_url: &str) -> PathBuf {
    home.join("trust")
        .join("allowed_signers")
        .join(format!("{}.signers", source_scope_id(source_url)))
}

fn trust_git_env(home: &Path, source_url: &str) -> Result<Vec<(String, String)>> {
    let path = trust_allowed_signers_path(home, source_url);
    let dir = home.join("trust").join("allowed_signers");
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    if !path.exists() {
        fs::write(&path, "").with_context(|| format!("failed to create {}", path.display()))?;
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

fn trust_git_output(
    home: &Path,
    source_url: &str,
    repo_dir: &Path,
    args: &[&str],
) -> Result<String> {
    let env = trust_git_env(home, source_url)?;
    git_output(repo_dir, args, &env)
}

fn trust_git_run(
    home: &Path,
    source_url: &str,
    repo_dir: &Path,
    args: &[&str],
    quiet: bool,
) -> Result<()> {
    let env = trust_git_env(home, source_url)?;
    git_run(repo_dir, args, &env, quiet)
}

fn commit_signing_key(home: &Path, source_url: &str, repo_dir: &Path) -> Option<String> {
    trust_git_output(home, source_url, repo_dir, &["log", "-1", "--format=%GK"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn commit_signing_fingerprint(home: &Path, source_url: &str, repo_dir: &Path) -> Option<String> {
    trust_git_output(home, source_url, repo_dir, &["log", "-1", "--format=%GF"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) fn repo_signing_keys(home: &Path, source_url: &str, repo_dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(v) = commit_signing_key(home, source_url, repo_dir).map(|s| normalize_key(&s)) {
        out.push(v);
    }
    if let Some(v) =
        commit_signing_fingerprint(home, source_url, repo_dir).map(|s| normalize_key(&s))
        && !out.iter().any(|x| x == &v)
    {
        out.push(v);
    }
    out
}

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
    Ok(render_policy_toml(&policy))
}

fn toml_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn render_string_array(values: &[String]) -> String {
    if values.is_empty() {
        return "[]".to_string();
    }
    let joined = values
        .iter()
        .map(|v| toml_quote(v))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{joined}]")
}

fn render_policy_toml(policy: &TrustPolicy) -> String {
    let mut out = String::new();
    out.push_str("[default]\n");
    out.push_str(&format!("allow = {}\n", policy.default.allow));
    out.push_str(&format!(
        "require_signed_commit = {}\n",
        policy.default.require_signed_commit
    ));
    out.push_str(&format!(
        "allowed_signing_keys = {}\n",
        render_string_array(&policy.default.allowed_signing_keys)
    ));
    for rule in &policy.rules {
        out.push_str("\n[[rules]]\n");
        if let Some(prefix) = rule.match_prefix.as_deref() {
            out.push_str(&format!("match_prefix = {}\n", toml_quote(prefix)));
        }
        out.push_str(&format!("allow = {}\n", rule.allow));
        out.push_str(&format!(
            "require_signed_commit = {}\n",
            rule.require_signed_commit
        ));
        out.push_str(&format!(
            "allowed_signing_keys = {}\n",
            render_string_array(&rule.allowed_signing_keys)
        ));
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrustListScope {
    All,
    Global,
    Local,
}

fn format_rule_block(title: &str, rule: &TrustRule) -> String {
    let mut out = String::new();
    out.push_str(title);
    out.push('\n');
    out.push_str(&format!("  allow: {}\n", rule.allow));
    out.push_str(&format!(
        "  require_signed_commit: {}\n",
        rule.require_signed_commit
    ));
    out.push_str("  allowed_signing_keys:\n");
    if rule.allowed_signing_keys.is_empty() {
        out.push_str("    - (none)\n");
    } else {
        for key in &rule.allowed_signing_keys {
            out.push_str(&format!("    - {}\n", key));
        }
    }
    out
}

fn source_url_from_filter(home: &Path, app_or_source: &str) -> Result<String> {
    if app_or_source.contains("://") || app_or_source.starts_with("git@") {
        return Ok(app_or_source.to_string());
    }
    let layout = crate::layout::resolve_installed_layout(home, app_or_source)?;
    let meta: crate::types::InstallMetadata = read_toml(&layout.meta_file)?;
    Ok(meta.source_url)
}

pub(crate) fn list_policy(home: &Path, scope: TrustListScope, app: Option<&str>) -> Result<String> {
    let policy = load_policy_or_default(home)?;
    let mut out = String::new();

    if let Some(app_filter) = app {
        let source_url = source_url_from_filter(home, app_filter)?;
        out.push_str(&format!("source: {}\n\n", source_url));
        if matches!(scope, TrustListScope::All | TrustListScope::Global) {
            out.push_str(&format_rule_block("scope: global", &policy.default));
            out.push('\n');
        }
        if matches!(scope, TrustListScope::All | TrustListScope::Local) {
            let mut matched: Vec<&TrustRule> = policy
                .rules
                .iter()
                .filter(|r| {
                    r.match_prefix
                        .as_deref()
                        .is_some_and(|prefix| source_url.starts_with(prefix))
                })
                .collect();
            matched.sort_by(|a, b| {
                let al = a.match_prefix.as_deref().map(str::len).unwrap_or(0);
                let bl = b.match_prefix.as_deref().map(str::len).unwrap_or(0);
                bl.cmp(&al)
            });
            if matched.is_empty() {
                out.push_str("scope: local\n  (no matching rules)\n");
            } else {
                for rule in matched {
                    let prefix = rule.match_prefix.as_deref().unwrap_or("-");
                    out.push_str(&format_rule_block(
                        &format!("scope: local ({prefix})"),
                        rule,
                    ));
                }
            }
            out.push('\n');
        }
        if matches!(scope, TrustListScope::All) {
            let effective = best_rule(&policy, &source_url);
            if let Some(prefix) = effective.match_prefix.as_deref() {
                out.push_str(&format!("effective_scope: local ({prefix})\n"));
            } else {
                out.push_str("effective_scope: global\n");
            }
        }
        return Ok(out.trim_end().to_string());
    }

    if matches!(scope, TrustListScope::All | TrustListScope::Global) {
        out.push_str(&format_rule_block("scope: global", &policy.default));
        out.push('\n');
    }
    if matches!(scope, TrustListScope::All | TrustListScope::Local) {
        if policy.rules.is_empty() {
            out.push_str("scope: local\n  (no rules)\n");
        } else {
            let mut rules: Vec<&TrustRule> = policy.rules.iter().collect();
            rules.sort_by(|a, b| a.match_prefix.cmp(&b.match_prefix));
            for rule in rules {
                let prefix = rule.match_prefix.as_deref().unwrap_or("-");
                out.push_str(&format_rule_block(
                    &format!("scope: local ({prefix})"),
                    rule,
                ));
            }
        }
    }
    Ok(out.trim_end().to_string())
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
    source_url: &str,
    repo_dir: &Path,
    match_prefix: Option<&str>,
) -> Result<usize> {
    let keys = repo_signing_keys(home, source_url, repo_dir);
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
    let added = append_missing_keys(rule, &keys);
    save_policy(home, &policy)?;
    Ok(added)
}

fn append_missing_keys(rule: &mut TrustRule, keys: &[String]) -> usize {
    let mut added = 0usize;
    for key in keys {
        if !rule
            .allowed_signing_keys
            .iter()
            .any(|k| normalize_key(k) == *key)
        {
            rule.allowed_signing_keys.push(key.clone());
            added += 1;
        }
    }
    added
}

fn rule_for_source<'a>(policy: &'a TrustPolicy, source_url: &str) -> &'a TrustRule {
    best_rule(policy, source_url)
}

fn keys_missing_for_trust_scope(
    home: &Path,
    source_url: &str,
    keys: &[String],
    global_scope: bool,
) -> Result<Vec<String>> {
    let policy = load_policy_or_default(home)?;
    let rule = if global_scope {
        &policy.default
    } else {
        rule_for_source(&policy, source_url)
    };
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

fn head_assessment(home: &Path, source_url: &str, repo_dir: &Path) -> Option<HeadAssessment> {
    let fmt = "%H%n%h%n%an%n%ae%n%aI%n%s%n%G?%n%GS%n%GK%n%GF";
    let raw = trust_git_output(
        home,
        source_url,
        repo_dir,
        &["log", "-1", &format!("--format={fmt}")],
    )
    .ok()?;
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
    global_scope: bool,
) -> Result<()> {
    let keys = repo_signing_keys(home, source_url, repo_dir);
    if keys.is_empty() {
        eprintln!(
            "[bmx][trust] no signer key/fingerprint found on HEAD (nothing to import): {}",
            repo_dir.display()
        );
        return Ok(());
    }
    let missing = keys_missing_for_trust_scope(home, source_url, &keys, global_scope)?;
    if missing.is_empty() {
        eprintln!("[bmx][trust] signer key already trusted for {source_url}");
        return Ok(());
    }

    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        bail!(
            "--trust requires an interactive terminal to confirm signer key import for {source_url}"
        );
    }

    let assessment = head_assessment(home, source_url, repo_dir);
    eprintln!("[bmx][trust] source: {source_url}");
    eprintln!("[bmx][trust] repo: {}", repo_dir.display());
    eprintln!(
        "[bmx][trust] policy scope: {}",
        if global_scope {
            "<default/global>"
        } else {
            "<source-specific>"
        }
    );
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
        "[bmx][trust] add these keys to trust policy? [y/N]: "
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
        if global_scope {
            add_allowed_signing_key(home, key, None)?;
        } else {
            add_allowed_signing_key(home, key, Some(source_url))?;
        }
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

fn signer_matches_allowed(
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

    let assessment = head_assessment(home, source_url, repo_dir);
    let sig_status = assessment
        .as_ref()
        .map(|a| a.sig_status.as_str())
        .unwrap_or_default();
    let good_signature = sig_status == "G";
    let trusted_signer = signer_matches_allowed(home, source_url, rule, repo_dir);
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
    use crate::io::write_toml;
    use crate::types::InstallMetadata;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

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
            .expect("git should run");
        assert!(status.success(), "git {:?} failed", args);
    }

    fn init_unsigned_repo(root: &Path, name: &str) -> std::path::PathBuf {
        let repo = root.join(name);
        fs::create_dir_all(&repo).unwrap();
        run_git(&repo, &["init"]);
        fs::write(repo.join("README.md"), "x").unwrap();
        run_git(&repo, &["add", "."]);
        run_git(&repo, &["commit", "-m", "init"]);
        repo
    }

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

    #[test]
    fn trust_git_env_creates_local_allowed_signers_file() {
        let home = tempdir().expect("tempdir");
        let source = "https://github.com/acme/tool.git";
        let env = trust_git_env(home.path(), source).expect("trust_git_env");
        let path = trust_allowed_signers_path(home.path(), source);
        assert!(path.exists());
        assert!(env.iter().any(|(k, _)| k == "GIT_CONFIG_COUNT"));
        assert!(env.iter().any(|(k, _)| k == "GIT_CONFIG_KEY_0"));
        assert!(env.iter().any(|(k, _)| k == "GIT_CONFIG_VALUE_0"));
    }

    #[test]
    fn trust_allowed_signers_path_is_source_scoped() {
        let home = tempdir().expect("tempdir");
        let a = trust_allowed_signers_path(home.path(), "https://github.com/acme/a.git");
        let b = trust_allowed_signers_path(home.path(), "https://github.com/acme/b.git");
        assert_ne!(a, b);
    }

    #[test]
    fn add_key_set_allow_and_set_signed_roundtrip() {
        let home = tempdir().unwrap();
        let prefix = "https://github.com/acme/";
        add_allowed_signing_key(home.path(), "abcd1234", Some(prefix)).unwrap();
        add_allowed_signing_key(home.path(), "abcd1234", Some(prefix)).unwrap(); // dedupe
        set_allow(home.path(), prefix, false).unwrap();
        set_require_signed_commit(home.path(), prefix, true).unwrap();
        let policy = load_policy_or_default(home.path()).unwrap();
        let r = best_rule(&policy, "https://github.com/acme/tool.git");
        assert_eq!(r.allowed_signing_keys.len(), 1);
        assert!(!r.allow);
        assert!(r.require_signed_commit);
    }

    #[test]
    fn enforce_source_trust_blocked_and_signed_requirements() {
        let home = tempdir().unwrap();
        let repos = tempdir().unwrap();
        let repo = init_unsigned_repo(repos.path(), "r1");

        set_allow(home.path(), "file://", false).unwrap();
        let e = enforce_source_trust(home.path(), "file://x", &repo).unwrap_err();
        assert!(e.to_string().contains("blocked by trust policy"));

        set_allow(home.path(), "file://", true).unwrap();
        set_require_signed_commit(home.path(), "file://", true).unwrap();
        assert!(enforce_source_trust(home.path(), "file://x", &repo).is_err());
    }

    #[test]
    fn import_signing_keys_from_unsigned_repo_fails() {
        let home = tempdir().unwrap();
        let repos = tempdir().unwrap();
        let repo = init_unsigned_repo(repos.path(), "r2");
        assert!(
            import_signing_keys_from_repo(home.path(), "file://x", &repo, Some("file://x"))
                .is_err()
        );
    }

    #[test]
    fn list_policy_for_source_without_local_rules_reports_global_effective_scope() {
        let home = tempdir().unwrap();
        let mut policy = TrustPolicy::default();
        policy.default.allowed_signing_keys = vec!["GLOBAL".into()];
        save_policy(home.path(), &policy).unwrap();
        let out = list_policy(
            home.path(),
            TrustListScope::All,
            Some("https://example.com/tool.git"),
        )
        .unwrap();
        assert!(out.contains("effective_scope: global"));
    }

    #[test]
    fn list_policy_scopes_and_app_filter() {
        let home = tempdir().expect("home");
        fs::create_dir_all(home.path()).unwrap();
        let mut policy = TrustPolicy::default();
        policy.default.allowed_signing_keys = vec!["GLOBAL".into()];
        policy.rules.push(TrustRule {
            match_prefix: Some("https://github.com/acme/".into()),
            allow: true,
            require_signed_commit: true,
            allowed_signing_keys: vec!["LOCAL".into()],
        });
        save_policy(home.path(), &policy).unwrap();

        let all = list_policy(home.path(), TrustListScope::All, None).unwrap();
        assert!(all.contains("scope: global"));
        assert!(all.contains("scope: local"));

        let global = list_policy(home.path(), TrustListScope::Global, None).unwrap();
        assert!(global.contains("GLOBAL"));
        assert!(!global.contains("scope: local"));

        let local = list_policy(home.path(), TrustListScope::Local, None).unwrap();
        assert!(local.contains("scope: local"));
        assert!(!local.contains("scope: global"));

        let layout = crate::layout::app_layout_for_id(home.path(), "my-app");
        fs::create_dir_all(layout.repo_dir.parent().unwrap()).unwrap();
        write_toml(
            &layout.meta_file,
            &InstallMetadata {
                app: "my-app".into(),
                source_url: "https://github.com/acme/tool.git".into(),
                executable_rel: "bin/tool".into(),
                strategy: "make".into(),
                requested_ref: None,
                resolved_commit: None,
                rust_package: None,
            },
        )
        .unwrap();
        let filtered = list_policy(home.path(), TrustListScope::All, Some("my-app")).unwrap();
        assert!(filtered.contains("effective_scope: local"));
        assert!(filtered.contains("LOCAL"));
    }

    #[test]
    fn append_missing_keys_counts_only_new_values() {
        let mut rule = TrustRule {
            match_prefix: None,
            allow: true,
            require_signed_commit: false,
            allowed_signing_keys: vec!["ABCD1234".into()],
        };
        let keys = vec!["ABCD1234".to_string(), "EFGH5678".to_string()];
        let added = append_missing_keys(&mut rule, &keys);
        assert_eq!(added, 1);
        assert_eq!(rule.allowed_signing_keys.len(), 2);
        assert!(rule.allowed_signing_keys.iter().any(|k| k == "ABCD1234"));
        assert!(rule.allowed_signing_keys.iter().any(|k| k == "EFGH5678"));
    }

    #[test]
    fn save_policy_handles_fingerprint_and_quoted_values() {
        let home = tempdir().expect("home");
        let mut policy = TrustPolicy::default();
        policy.default.allowed_signing_keys = vec![
            "SHA256:6N4WHCBUBFBLXJ1PB+JCWNANCC0NDIX/TYCUDJRVDJO".into(),
            "KEY\"WITH\\ESCAPES".into(),
        ];
        policy.rules.push(TrustRule {
            match_prefix: Some("https://github.com/acme/tool.git".into()),
            allow: true,
            require_signed_commit: true,
            allowed_signing_keys: vec!["ABCD1234".into()],
        });

        save_policy(home.path(), &policy).expect("save policy");
        let loaded = load_policy_or_default(home.path()).expect("reload policy");
        assert_eq!(
            loaded.default.allowed_signing_keys,
            policy.default.allowed_signing_keys
        );
        assert_eq!(loaded.rules.len(), 1);
        assert_eq!(
            loaded.rules[0].match_prefix.as_deref(),
            Some("https://github.com/acme/tool.git")
        );
    }
}
