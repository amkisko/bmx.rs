use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::trust_feed::load_compromised_entries;

#[derive(Debug, Deserialize, Default)]
struct LocalTrustPolicy {
    #[serde(default)]
    default: LocalTrustRule,
    #[serde(default)]
    rules: Vec<LocalTrustRule>,
}

#[derive(Debug, Deserialize, Default)]
struct LocalTrustRule {
    #[serde(default)]
    match_prefix: Option<String>,
    #[serde(default)]
    allowed_signing_keys: Vec<String>,
}

fn normalize_key(s: &str) -> String {
    s.trim().to_ascii_uppercase()
}

fn trust_policy_path(home: &Path) -> PathBuf {
    home.join("trust.toml")
}

fn read_local_trusted_keys(home: &Path) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let path = trust_policy_path(home);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed reading {}", path.display()))?;
    let policy: LocalTrustPolicy =
        toml::from_str(&content).with_context(|| format!("failed parsing {}", path.display()))?;

    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for key in policy.default.allowed_signing_keys {
        let k = normalize_key(&key);
        if !k.is_empty() {
            out.entry(k)
                .or_default()
                .insert("global/default".to_string());
        }
    }
    for rule in policy.rules {
        let scope = format!(
            "local:{}",
            rule.match_prefix.unwrap_or_else(|| "-".to_string())
        );
        for key in rule.allowed_signing_keys {
            let k = normalize_key(&key);
            if !k.is_empty() {
                out.entry(k).or_default().insert(scope.clone());
            }
        }
    }
    Ok(out)
}

pub(crate) fn run_trust_check(home: &Path, source: Option<&str>) -> Result<()> {
    let trusted = read_local_trusted_keys(home)?;
    if trusted.is_empty() {
        println!(
            "no trusted signing keys found in {}",
            trust_policy_path(home).display()
        );
        return Ok(());
    }

    let (source_desc, compromised_entries) = load_compromised_entries(home, source)?;
    let mut compromised_by_key = HashMap::new();
    for e in compromised_entries {
        compromised_by_key
            .entry(e.key.clone())
            .or_insert_with(Vec::new)
            .push(e);
    }

    let mut hit_count = 0usize;
    for (key, scopes) in &trusted {
        let Some(entries) = compromised_by_key.get(key) else {
            continue;
        };
        hit_count += 1;
        println!("[compromised] {key}");
        println!(
            "  scopes: {}",
            scopes.iter().cloned().collect::<Vec<_>>().join(", ")
        );
        for e in entries {
            if let Some(reason) = &e.reason {
                println!("  reason: {reason}");
            }
            if let Some(reference) = &e.reference {
                println!("  reference: {reference}");
            }
            if let Some(reported_at) = &e.reported_at {
                println!("  reported_at: {reported_at}");
            }
        }
    }

    println!(
        "checked {} trusted key(s) against {} compromised key record(s) from {}",
        trusted.len(),
        compromised_by_key.len(),
        source_desc
    );
    if hit_count > 0 {
        bail!("found {hit_count} compromised trusted key(s)");
    }
    println!("no compromised trusted signing keys detected");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn trusted_keys_are_collected_from_global_and_local_rules() {
        let home = tempdir().expect("home");
        fs::write(
            home.path().join("trust.toml"),
            "[default]\nallowed_signing_keys=[\"SHA256:AAA\"]\n\n[[rules]]\nmatch_prefix=\"https://github.com/acme/\"\nallowed_signing_keys=[\"SHA256:BBB\",\"SHA256:AAA\"]\n",
        )
        .expect("write trust policy");
        let keys = read_local_trusted_keys(home.path()).expect("read keys");
        assert_eq!(keys.len(), 2);
        assert!(keys.get("SHA256:AAA").is_some_and(|s| s.len() == 2));
        assert!(keys.get("SHA256:BBB").is_some_and(|s| s.len() == 1));
    }

    #[test]
    fn run_trust_check_fails_on_compromised_match() {
        let home = tempdir().expect("home");
        fs::write(
            home.path().join("trust.toml"),
            "[default]\nallowed_signing_keys=[\"SHA256:AAA\"]\n",
        )
        .expect("write trust policy");
        let feed = home.path().join("compromised-keys.txt");
        fs::write(&feed, "SHA256:AAA # leaked").expect("write feed");
        let err =
            run_trust_check(home.path(), Some(feed.to_str().expect("utf8 path"))).unwrap_err();
        assert!(
            err.to_string()
                .contains("found 1 compromised trusted key(s)")
        );
    }
}
