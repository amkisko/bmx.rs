use std::path::Path;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::io::read_toml;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub(crate) struct TrustPolicy {
    #[serde(default)]
    pub(crate) default: TrustRule,
    #[serde(default)]
    pub(crate) rules: Vec<TrustRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct TrustRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) match_prefix: Option<String>,
    #[serde(default = "default_allow")]
    pub(crate) allow: bool,
    #[serde(default)]
    pub(crate) require_signed_commit: bool,
    #[serde(default)]
    pub(crate) allowed_signing_keys: Vec<String>,
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

pub(crate) fn load_policy(home: &Path) -> Result<Option<TrustPolicy>> {
    let path = trust_policy_path(home);
    if !path.exists() {
        return Ok(None);
    }
    let policy = read_toml::<TrustPolicy>(&path)
        .with_context(|| format!("failed to load trust policy {}", path.display()))?;
    Ok(Some(policy))
}

pub(crate) fn load_policy_or_default(home: &Path) -> Result<TrustPolicy> {
    Ok(load_policy(home)?.unwrap_or_default())
}

pub(crate) fn save_policy(home: &Path, policy: &TrustPolicy) -> Result<()> {
    let path = trust_policy_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&path, render_policy_toml(policy))
        .with_context(|| format!("failed to write {}", path.display()))
}

pub(crate) fn best_rule<'a>(policy: &'a TrustPolicy, source_url: &str) -> &'a TrustRule {
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

pub(crate) fn normalize_key(s: &str) -> String {
    s.trim().to_ascii_uppercase()
}

pub(crate) fn source_scope_id(source_url: &str) -> String {
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

pub(crate) fn mutable_rule_for_match_prefix<'a>(
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

pub(crate) fn render_policy_toml(policy: &TrustPolicy) -> String {
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

pub(crate) fn format_rule_block(title: &str, rule: &TrustRule) -> String {
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

pub(crate) fn source_url_from_filter(home: &Path, app_or_source: &str) -> Result<String> {
    if app_or_source.contains("://") || app_or_source.starts_with("git@") {
        return Ok(app_or_source.to_string());
    }
    let layout = crate::layout::resolve_installed_layout(home, app_or_source)?;
    let meta: crate::types::InstallMetadata = read_toml(&layout.meta_file)?;
    Ok(meta.source_url)
}

pub(crate) fn append_missing_keys(rule: &mut TrustRule, keys: &[String]) -> usize {
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

pub(crate) fn rule_for_source<'a>(policy: &'a TrustPolicy, source_url: &str) -> &'a TrustRule {
    best_rule(policy, source_url)
}

pub(crate) fn keys_missing_for_trust_scope(
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
