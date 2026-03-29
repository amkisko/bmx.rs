use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use url::Url;

use crate::config::load_config;
use crate::repo::sync_repo;

pub(crate) const DEFAULT_COMPROMISED_KEYS_SOURCE: &str =
    "https://raw.githubusercontent.com/bmx-rs/trust-lists/main/compromised-keys.toml";

const REPO_CANDIDATE_FILES: &[&str] = &[
    "compromised-keys.toml",
    "compromised_keys.toml",
    "trust/compromised-keys.toml",
    "trust/compromised_keys.toml",
    "compromised-keys.txt",
    "compromised_keys.txt",
    "trust/compromised-keys.txt",
    "trust/compromised_keys.txt",
];

#[derive(Debug, Clone)]
pub(crate) struct CompromisedEntry {
    pub(crate) key: String,
    pub(crate) reason: Option<String>,
    pub(crate) reference: Option<String>,
    pub(crate) reported_at: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct CompromisedTomlFeed {
    #[serde(default)]
    keys: Vec<CompromisedTomlEntry>,
}

#[derive(Debug, Deserialize, Default)]
struct CompromisedTomlEntry {
    #[serde(default)]
    value: String,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    reference: Option<String>,
    #[serde(default)]
    reported_at: Option<String>,
}

fn normalize_key(s: &str) -> String {
    s.trim().to_ascii_uppercase()
}

fn is_git_source(source: &str) -> bool {
    let s = source.trim().to_ascii_lowercase();
    s.starts_with("git@")
        || s.starts_with("ssh://")
        || s.starts_with("git://")
        || s.ends_with(".git")
}

fn read_http_body(url: &str) -> Result<String> {
    let resp = ureq::get(url)
        .set(
            "User-Agent",
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .with_context(|| format!("HTTP request failed for {url}"))?;
    let status = resp.status();
    let body = resp.into_string().unwrap_or_default();
    if !(200..300).contains(&status) {
        bail!("compromised-key source returned HTTP {status}");
    }
    Ok(body)
}

fn parse_compromised_txt(body: &str) -> Vec<CompromisedEntry> {
    let mut out = Vec::new();
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, '#');
        let head = parts.next().unwrap_or_default().trim();
        if head.is_empty() {
            continue;
        }
        let key = normalize_key(head.split_whitespace().next().unwrap_or_default());
        if key.is_empty() {
            continue;
        }
        let reason = parts
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        out.push(CompromisedEntry {
            key,
            reason,
            reference: None,
            reported_at: None,
        });
    }
    out
}

fn parse_compromised_toml(body: &str) -> Result<Vec<CompromisedEntry>> {
    let feed: CompromisedTomlFeed = toml::from_str(body)
        .context("invalid compromised-key TOML; expected `[[keys]] value = \"<key>\"` entries")?;
    let mut out = Vec::new();
    for k in feed.keys {
        let key = normalize_key(&k.value);
        if key.is_empty() {
            continue;
        }
        out.push(CompromisedEntry {
            key,
            reason: k.reason,
            reference: k.reference,
            reported_at: k.reported_at,
        });
    }
    Ok(out)
}

fn parse_source_body(source: &str, body: &str) -> Result<Vec<CompromisedEntry>> {
    let lower = source.to_ascii_lowercase();
    if lower.ends_with(".txt") {
        return Ok(parse_compromised_txt(body));
    }
    if lower.ends_with(".toml") {
        return parse_compromised_toml(body);
    }
    parse_compromised_toml(body).or_else(|_| Ok(parse_compromised_txt(body)))
}

fn load_entries_from_repo(
    home: &Path,
    git_source: &str,
) -> Result<(String, Vec<CompromisedEntry>)> {
    let cfg = load_config(home)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_root =
        std::env::temp_dir().join(format!("bmx-trust-check-{}-{}", std::process::id(), stamp));
    let repo_dir: PathBuf = tmp_root.join("repo");
    fs::create_dir_all(&repo_dir)?;

    let result = (|| -> Result<(String, Vec<CompromisedEntry>)> {
        sync_repo(git_source, &repo_dir, &cfg, false)?;
        for rel in REPO_CANDIDATE_FILES {
            let p = repo_dir.join(rel);
            if !p.exists() {
                continue;
            }
            let body = fs::read_to_string(&p)
                .with_context(|| format!("failed reading {}", p.display()))?;
            return Ok((
                format!("{git_source}#{rel}"),
                parse_source_body(rel, &body)?,
            ));
        }
        bail!(
            "no compromised key feed found in git source; expected one of: {}",
            REPO_CANDIDATE_FILES.join(", ")
        );
    })();

    let _ = fs::remove_dir_all(&tmp_root);
    result
}

pub(crate) fn load_compromised_entries(
    home: &Path,
    source: Option<&str>,
) -> Result<(String, Vec<CompromisedEntry>)> {
    let source = source.unwrap_or(DEFAULT_COMPROMISED_KEYS_SOURCE).trim();
    if source.is_empty() {
        bail!("compromised-key source is empty");
    }

    if is_git_source(source) {
        return load_entries_from_repo(home, source);
    }

    if source.starts_with("http://") || source.starts_with("https://") {
        let body = read_http_body(source)?;
        return Ok((source.to_string(), parse_source_body(source, &body)?));
    }

    if source.starts_with("file://") {
        let url = Url::parse(source).context("invalid file:// URL")?;
        let path = url
            .to_file_path()
            .map_err(|_| anyhow!("invalid file:// path in source"))?;
        let body = fs::read_to_string(&path)
            .with_context(|| format!("failed reading {}", path.display()))?;
        return Ok((source.to_string(), parse_source_body(source, &body)?));
    }

    let path = Path::new(source);
    if path.exists() {
        let body = fs::read_to_string(path)
            .with_context(|| format!("failed reading {}", path.display()))?;
        return Ok((
            path.display().to_string(),
            parse_source_body(source, &body)?,
        ));
    }

    bail!(
        "unsupported trust source `{source}`; use a git URL, http(s) .txt/.toml URL, file:// URL, or local file path"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_txt_feed_supports_comments() {
        let entries = parse_compromised_txt("\n# list\nSHA256:AAA # leaked\n\nSHA256:BBB\n");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "SHA256:AAA");
        assert_eq!(entries[0].reason.as_deref(), Some("leaked"));
    }

    #[test]
    fn parse_toml_feed_reads_structured_fields() {
        let body = r#"
[[keys]]
value = "SHA256:AAA"
reason = "private key leak"
reference = "https://example.test/advisory"
reported_at = "2026-01-01"
"#;
        let entries = parse_compromised_toml(body).expect("parse toml");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "SHA256:AAA");
        assert_eq!(entries[0].reason.as_deref(), Some("private key leak"));
        assert_eq!(
            entries[0].reference.as_deref(),
            Some("https://example.test/advisory")
        );
        assert_eq!(entries[0].reported_at.as_deref(), Some("2026-01-01"));
    }

    #[test]
    fn parse_source_body_auto_falls_back_to_txt() {
        let entries = parse_source_body("https://example.test/feed", "SHA256:CCC").expect("parse");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "SHA256:CCC");
    }
}
