use std::path::Path;

use anyhow::{Result, bail};

use super::git_env::repo_signing_keys;
use super::policy::{
    TrustRule, append_missing_keys, best_rule, format_rule_block, load_policy_or_default,
    mutable_rule_for_match_prefix, normalize_key, render_policy_toml, save_policy,
    source_url_from_filter,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrustListScope {
    All,
    Global,
    Local,
}

pub(crate) fn show_policy_toml(home: &Path) -> Result<String> {
    let policy = load_policy_or_default(home)?;
    Ok(render_policy_toml(&policy))
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

pub(crate) fn remove_allowed_signing_key(
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
    let before = rule.allowed_signing_keys.len();
    rule.allowed_signing_keys
        .retain(|k| normalize_key(k) != key);
    if rule.allowed_signing_keys.len() == before {
        bail!(
            "signing key not found in allowed_signing_keys for {}",
            match_prefix.unwrap_or("<default>")
        );
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
