use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use url::Url;

use crate::source::resolve_source;
use crate::types::Config;

use super::http::{clip, http_get_json, minimal_headers};

/// AUR RPC: every result is a named package base with a public PKGBUILD git URL — no extra probe.
pub(crate) fn search_aur(q: &str, limit: usize, print_url: bool) -> Result<()> {
    let mut url = Url::parse("https://aur.archlinux.org/rpc")?;
    url.query_pairs_mut()
        .append_pair("v", "5")
        .append_pair("type", "search")
        .append_pair("arg", q.trim());

    let body = http_get_json(&url, minimal_headers)?;
    let parsed: AurRpcResponse = serde_json::from_str(&body).with_context(|| {
        format!(
            "unexpected AUR RPC response (first 200 chars): {}",
            clip(&body)
        )
    })?;

    for r in parsed.results.into_iter().take(limit) {
        let clone_url = format!("https://aur.archlinux.org/{}.git", r.package_base);
        if print_url {
            println!("{clone_url}");
        } else {
            println!("{}\t{clone_url}", r.name);
        }
    }
    Ok(())
}

#[derive(Deserialize)]
pub(crate) struct AurRpcResponse {
    #[serde(default)]
    pub(crate) results: Vec<AurResult>,
}

#[derive(Deserialize)]
pub(crate) struct AurResult {
    #[serde(rename = "Name")]
    pub(crate) name: String,
    #[serde(rename = "PackageBase")]
    pub(crate) package_base: String,
}

pub(crate) fn search_homebrew(q: &str, limit: usize, print_url: bool) -> Result<()> {
    let url = Url::parse("https://formulae.brew.sh/api/formula.json").expect("static URL");
    let body = http_get_json(&url, minimal_headers)?;
    let formulas: Vec<JsonValue> = serde_json::from_str(&body).with_context(|| {
        format!(
            "unexpected Homebrew JSON (first 200 chars): {}",
            clip(&body)
        )
    })?;

    let needle = q.trim().to_ascii_lowercase();
    let mut printed = 0usize;
    for f in formulas {
        if printed >= limit {
            break;
        }
        let Some(name) = f.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        if !name.to_ascii_lowercase().contains(&needle) {
            continue;
        }
        let Some(spec) = brew_formula_to_install_spec(&f) else {
            continue;
        };
        printed += 1;
        if print_url {
            println!("{spec}");
        } else {
            println!("{name}\t{spec}");
        }
    }
    Ok(())
}

/// Pick a git clone URL from formula metadata (homepage or stable tarball URL host path).
#[allow(clippy::collapsible_if)]
pub(crate) fn brew_formula_to_install_spec(f: &JsonValue) -> Option<String> {
    if let Some(h) = f.get("homepage").and_then(|h| h.as_str()) {
        if let Some(s) = homepage_to_git_clone_spec(h) {
            return Some(s);
        }
    }
    if let Some(u) = f
        .get("urls")
        .and_then(|u| u.get("stable"))
        .and_then(|s| s.get("url"))
        .and_then(|u| u.as_str())
    {
        return extract_git_clone_from_archive_url(u);
    }
    None
}

pub(crate) fn homepage_to_git_clone_spec(homepage: &str) -> Option<String> {
    let u = Url::parse(homepage.trim()).ok()?;
    let host = u.host_str()?.to_ascii_lowercase();
    if host == "github.com" || host == "www.github.com" {
        let mut segs = u.path_segments()?;
        let a = segs.next()?;
        let b = segs.next()?;
        if a.is_empty() || b.is_empty() {
            return None;
        }
        return Some(format!("https://github.com/{a}/{b}.git"));
    }
    if host == "gitlab.com" || host.ends_with(".gitlab.com") {
        let mut segs = u.path_segments()?;
        let a = segs.next()?;
        let b = segs.next()?;
        if a.is_empty() || b.is_empty() {
            return None;
        }
        return Some(format!("https://{host}/{a}/{b}.git"));
    }
    None
}

pub(crate) fn extract_git_clone_from_archive_url(u: &str) -> Option<String> {
    let url = Url::parse(u).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    if host != "github.com" {
        return None;
    }
    let mut segs = url.path_segments()?;
    let owner = segs.next()?;
    let repo = segs.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("https://github.com/{owner}/{repo}.git"))
}

pub(crate) fn install_hint_resolves(cfg: &Config, hint: &str) -> Result<bool> {
    match resolve_source(cfg, hint) {
        Ok(url) => {
            let u = url.to_ascii_lowercase();
            Ok(u.ends_with(".git") || looks_like_any_git_remote(&u))
        }
        Err(_) => Ok(false),
    }
}

pub(crate) fn looks_like_any_git_remote(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("git://")
        || url.starts_with("ssh://")
        || url.starts_with("git@")
}
