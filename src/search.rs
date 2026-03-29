//! Search for installable sources without cloning: GitHub/GitLab APIs plus optional
//! root-directory checks (same markers as `detect_strategy`), AUR RPC, and Homebrew
//! formula metadata (`formulae.brew.sh`).

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use url::Url;

use crate::cli::SearchBackendArg;
use crate::config::load_config;
use crate::source::{normalize_explicit_url, normalize_source_base, resolve_source};
use crate::types::Config;

const DEFAULT_UA: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Root files bmx build detection looks for (see `build::BUILD_PLUGINS`).
const ROOT_BUILD_MARKERS: &[&str] = &[
    "Cargo.toml",
    "CMakeLists.txt",
    "PKGBUILD",
    "Brewfile",
    "Makefile",
    "makefile",
    "bmx.toml",
];

pub(crate) fn run_search(
    home: &Path,
    query: &[String],
    backend: SearchBackendArg,
    limit: usize,
    include_forks: bool,
    print_url: bool,
    probe: bool,
) -> Result<()> {
    let q = query.join(" ");
    if q.trim().is_empty() {
        bail!("search query is empty");
    }
    let limit = limit.clamp(1, 100);
    let cfg = load_config(home)?;

    match backend {
        SearchBackendArg::Aur => search_aur(&q, limit, print_url),
        SearchBackendArg::Homebrew => search_homebrew(&q, limit, print_url),
        SearchBackendArg::Github => {
            let api_root = std::env::var("BMX_GITHUB_API_BASE")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().trim_end_matches('/').to_string())
                .unwrap_or_else(|| "https://api.github.com".to_string());
            let cfg_resolve = config_with_github_base(&cfg);
            search_github(
                &cfg_resolve,
                &api_root,
                &q,
                limit,
                include_forks,
                print_url,
                probe,
            )
        }
        SearchBackendArg::Gitlab => {
            let base_raw = cfg
                .default_source
                .as_deref()
                .ok_or_else(|| anyhow!("default source is not configured; run `bmx source set-default <url>` (GitLab origin)"))?;
            let base_url = Url::parse(&normalize_source_base(base_raw))
                .context("invalid default_source URL")?;
            let api_root = gitlab_api_v4_root(&base_url);
            let cfg_resolve = config_with_gitlab_base(&cfg, &base_url);
            search_gitlab(
                &cfg_resolve,
                &api_root,
                &q,
                limit,
                include_forks,
                print_url,
                probe,
            )
        }
        SearchBackendArg::Auto => {
            if let Ok(v) = std::env::var("BMX_GITHUB_API_BASE") {
                let v = v.trim().trim_end_matches('/');
                if !v.is_empty() {
                    return search_github(&cfg, v, &q, limit, include_forks, print_url, probe);
                }
            }

            let base_raw = cfg.default_source.as_deref().ok_or_else(|| {
                anyhow!("default source is not configured; run `bmx source set-default <url>`")
            })?;
            let base_url = Url::parse(&normalize_source_base(base_raw))
                .context("invalid default_source URL")?;
            let host = base_url
                .host_str()
                .ok_or_else(|| anyhow!("default_source has no host"))?
                .to_ascii_lowercase();

            let is_github = matches!(host.as_str(), "github.com" | "www.github.com")
                || host.ends_with(".github.com");
            let is_gitlab = host.contains("gitlab") || host.ends_with(".gitlab.com");

            if is_github {
                let api_root = if matches!(host.as_str(), "github.com" | "www.github.com") {
                    "https://api.github.com".to_string()
                } else {
                    format!("{}://{}/api/v3", base_url.scheme(), host)
                };
                search_github(&cfg, &api_root, &q, limit, include_forks, print_url, probe)
            } else if is_gitlab {
                let api_root = gitlab_api_v4_root(&base_url);
                search_gitlab(&cfg, &api_root, &q, limit, include_forks, print_url, probe)
            } else {
                bail!(
                    "search --backend auto needs github.com or gitlab.com in default_source, or use --backend aur|homebrew|github|gitlab.\n\
                     current host: {host}"
                );
            }
        }
    }
}

fn config_with_github_base(cfg: &Config) -> Config {
    let mut c = cfg.clone();
    c.default_source = Some("https://github.com".to_string());
    c
}

fn config_with_gitlab_base(cfg: &Config, site: &Url) -> Config {
    let mut c = cfg.clone();
    let scheme = site.scheme();
    let host = site.host_str().unwrap_or("");
    let port = site.port().map(|p| format!(":{p}")).unwrap_or_default();
    c.default_source = Some(format!("{scheme}://{host}{port}"));
    c
}

fn gitlab_api_v4_root(site: &Url) -> String {
    let scheme = site.scheme();
    let host = site.host_str().unwrap_or("");
    let port = site.port().map(|p| format!(":{p}")).unwrap_or_default();
    format!("{scheme}://{host}{port}/api/v4")
}

fn github_qualifiers(q: &str, include_forks: bool) -> String {
    let mut out = q.trim().to_string();
    if !out.contains("archived:") {
        out.push_str(" archived:false");
    }
    if !include_forks && !out.contains("fork:") {
        out.push_str(" fork:false");
    }
    out
}

fn search_github(
    cfg: &Config,
    api_root: &str,
    query: &str,
    limit: usize,
    include_forks: bool,
    print_url: bool,
    probe: bool,
) -> Result<()> {
    let q = github_qualifiers(query, include_forks);
    let base = api_root.trim_end_matches('/');
    let mut url = Url::parse(&format!("{base}/search/repositories"))
        .with_context(|| format!("invalid GitHub API root `{api_root}`"))?;
    url.query_pairs_mut()
        .append_pair("q", &q)
        .append_pair("per_page", &limit.to_string());

    let body = http_get_json(&url, github_headers)?;
    let parsed: GitHubSearchResponse = serde_json::from_str(&body).with_context(|| {
        format!(
            "unexpected GitHub API response (first 200 chars): {}",
            clip(&body)
        )
    })?;

    let mut printed = 0usize;
    for item in parsed.items {
        if printed >= limit {
            break;
        }
        if item.disabled {
            continue;
        }
        if !include_forks && item.fork {
            continue;
        }
        if item.archived {
            continue;
        }
        let owner_repo = item.full_name.clone();
        let parts: Vec<&str> = owner_repo.split('/').collect();
        if parts.len() != 2 {
            continue;
        }
        let (owner, repo) = (parts[0], parts[1]);
        if probe && !github_repo_root_buildable(base, owner, repo, item.default_branch.as_deref())?
        {
            continue;
        }
        if !install_hint_resolves(cfg, &owner_repo)? {
            continue;
        }
        printed += 1;
        if print_url {
            println!(
                "{}",
                item.clone_url.unwrap_or_else(|| {
                    normalize_explicit_url(&format!("https://github.com/{owner_repo}.git"))
                })
            );
        } else {
            println!("{owner_repo}");
        }
    }
    Ok(())
}

fn github_repo_root_buildable(
    api_root: &str,
    owner: &str,
    repo: &str,
    default_branch: Option<&str>,
) -> Result<bool> {
    let base = api_root.trim_end_matches('/');
    let mut url = Url::parse(&format!("{base}/repos/{owner}/{repo}/contents/"))?;
    if let Some(b) = default_branch.filter(|s| !s.is_empty()) {
        url.query_pairs_mut().append_pair("ref", b);
    }
    let body = match http_get_json_soft(&url, github_headers) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };
    Ok(github_contents_has_marker(&body))
}

fn github_contents_has_marker(body: &str) -> bool {
    let Ok(v) = serde_json::from_str::<JsonValue>(body) else {
        return false;
    };
    let names: Vec<String> = match v.as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|e| e.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect(),
        None => return false,
    };
    names
        .iter()
        .any(|n| ROOT_BUILD_MARKERS.contains(&n.as_str()) || n.ends_with(".rb"))
}

#[derive(Deserialize)]
struct GitHubSearchResponse {
    items: Vec<GitHubRepo>,
}

#[derive(Deserialize)]
struct GitHubRepo {
    full_name: String,
    clone_url: Option<String>,
    archived: bool,
    fork: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    default_branch: Option<String>,
}

fn search_gitlab(
    cfg: &Config,
    api_root: &str,
    query: &str,
    limit: usize,
    include_forks: bool,
    print_url: bool,
    probe: bool,
) -> Result<()> {
    let base = api_root.trim_end_matches('/');
    let mut url = Url::parse(&format!("{base}/projects"))
        .with_context(|| format!("invalid GitLab API root `{api_root}`"))?;
    url.query_pairs_mut()
        .append_pair("search", query.trim())
        .append_pair("per_page", &limit.to_string())
        .append_pair("archived", "false");

    let body = http_get_json(&url, gitlab_headers)?;
    let items: Vec<GitLabProject> = serde_json::from_str(&body).with_context(|| {
        format!(
            "unexpected GitLab API response (first 200 chars): {}",
            clip(&body)
        )
    })?;

    let mut printed = 0usize;
    for item in items {
        if printed >= limit {
            break;
        }
        if item.empty_repo == Some(true) {
            continue;
        }
        if !include_forks && item.fork == Some(true) {
            continue;
        }
        let path = item.path_with_namespace.clone();
        if probe && !gitlab_repo_root_buildable(base, item.id, item.default_branch.as_deref())? {
            continue;
        }
        if !install_hint_resolves(cfg, &path)? {
            continue;
        }
        printed += 1;
        if print_url {
            let u = item
                .http_url_to_repo
                .or(item.ssh_url_to_repo)
                .unwrap_or_else(|| {
                    normalize_explicit_url(&format!("https://gitlab.com/{path}.git"))
                });
            println!("{u}");
        } else {
            println!("{path}");
        }
    }
    Ok(())
}

fn gitlab_repo_root_buildable(
    api_root: &str,
    project_id: u64,
    default_branch: Option<&str>,
) -> Result<bool> {
    let base = api_root.trim_end_matches('/');
    let mut url = Url::parse(&format!("{base}/projects/{project_id}/repository/tree"))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("per_page", "100");
        if let Some(b) = default_branch.filter(|s| !s.is_empty()) {
            pairs.append_pair("ref", b);
        }
    }
    let body = match http_get_json_soft(&url, gitlab_headers) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };
    let Ok(entries) = serde_json::from_str::<Vec<GitLabTreeEntry>>(&body) else {
        return Ok(false);
    };
    Ok(entries
        .iter()
        .any(|e| ROOT_BUILD_MARKERS.contains(&e.name.as_str()) || e.name.ends_with(".rb")))
}

#[derive(Deserialize)]
struct GitLabTreeEntry {
    name: String,
}

#[derive(Deserialize)]
struct GitLabProject {
    id: u64,
    path_with_namespace: String,
    http_url_to_repo: Option<String>,
    ssh_url_to_repo: Option<String>,
    #[serde(default)]
    empty_repo: Option<bool>,
    #[serde(default)]
    fork: Option<bool>,
    #[serde(default)]
    default_branch: Option<String>,
}

/// AUR RPC: every result is a named package base with a public PKGBUILD git URL — no extra probe.
fn search_aur(q: &str, limit: usize, print_url: bool) -> Result<()> {
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
struct AurRpcResponse {
    #[serde(default)]
    results: Vec<AurResult>,
}

#[derive(Deserialize)]
struct AurResult {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "PackageBase")]
    package_base: String,
}

fn search_homebrew(q: &str, limit: usize, print_url: bool) -> Result<()> {
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
fn brew_formula_to_install_spec(f: &JsonValue) -> Option<String> {
    if let Some(h) = f.get("homepage").and_then(|h| h.as_str())
        && let Some(s) = homepage_to_git_clone_spec(h)
    {
        return Some(s);
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

fn homepage_to_git_clone_spec(homepage: &str) -> Option<String> {
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

fn extract_git_clone_from_archive_url(u: &str) -> Option<String> {
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

fn install_hint_resolves(cfg: &Config, hint: &str) -> Result<bool> {
    match resolve_source(cfg, hint) {
        Ok(url) => {
            let u = url.to_ascii_lowercase();
            Ok(u.ends_with(".git") || looks_like_any_git_remote(&u))
        }
        Err(_) => Ok(false),
    }
}

fn looks_like_any_git_remote(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("git://")
        || url.starts_with("ssh://")
        || url.starts_with("git@")
}

fn minimal_headers(_url: &Url) -> Vec<(&'static str, String)> {
    vec![("User-Agent", DEFAULT_UA.to_string())]
}

fn github_headers(_url: &Url) -> Vec<(&'static str, String)> {
    let mut h = vec![
        ("User-Agent", DEFAULT_UA.to_string()),
        ("Accept", "application/vnd.github+json".to_string()),
    ];
    if let Ok(tok) = std::env::var("GITHUB_TOKEN") {
        let tok = tok.trim();
        if !tok.is_empty() {
            h.push(("Authorization", format!("Bearer {tok}")));
        }
    }
    h
}

fn gitlab_headers(_url: &Url) -> Vec<(&'static str, String)> {
    let mut h = vec![("User-Agent", DEFAULT_UA.to_string())];
    if let Ok(tok) = std::env::var("GITLAB_TOKEN") {
        let tok = tok.trim();
        if !tok.is_empty() {
            h.push(("PRIVATE-TOKEN", tok.to_string()));
        }
    }
    h
}

fn http_get_json(url: &Url, headers: fn(&Url) -> Vec<(&'static str, String)>) -> Result<String> {
    let body = http_request_json(url, headers, true)?;
    Ok(body)
}

fn http_get_json_soft(
    url: &Url,
    headers: fn(&Url) -> Vec<(&'static str, String)>,
) -> Result<String> {
    http_request_json(url, headers, false)
}

fn http_request_json(
    url: &Url,
    headers: fn(&Url) -> Vec<(&'static str, String)>,
    bail_on_error: bool,
) -> Result<String> {
    let h = headers(url);
    let mut req = ureq::get(url.as_str()).set("User-Agent", DEFAULT_UA);
    for (k, v) in &h {
        if *k != "User-Agent" {
            req = req.set(k, v);
        }
    }
    let resp = req
        .call()
        .with_context(|| format!("HTTP request failed for {}", url.as_str()))?;
    let status = resp.status();
    let body = resp.into_string().unwrap_or_default();
    if !(200..300).contains(&status) {
        if bail_on_error {
            bail!("search API returned HTTP {status}: {}", clip(&body));
        }
        bail!("HTTP {}", status);
    }
    Ok(body)
}

fn clip(s: &str) -> String {
    s.chars().take(200).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_qualifiers_adds_archived_and_fork() {
        assert_eq!(
            github_qualifiers("ripgrep", false),
            "ripgrep archived:false fork:false"
        );
        assert_eq!(github_qualifiers("ripgrep", true), "ripgrep archived:false");
    }

    #[test]
    fn github_contents_detects_cargo() {
        let body = r#"[{"name":"Cargo.toml","type":"file"},{"name":"README.md","type":"file"}]"#;
        assert!(github_contents_has_marker(body));
    }

    #[test]
    fn github_contents_negative() {
        let body = r#"[{"name":"README.md","type":"file"}]"#;
        assert!(!github_contents_has_marker(body));
    }

    #[test]
    fn homepage_to_git_clone_spec_github() {
        assert_eq!(
            homepage_to_git_clone_spec("https://github.com/BurntSushi/ripgrep"),
            Some("https://github.com/BurntSushi/ripgrep.git".into())
        );
    }

    #[test]
    fn extract_git_from_archive() {
        assert_eq!(
            extract_git_clone_from_archive_url(
                "https://github.com/BurntSushi/ripgrep/archive/refs/tags/14.1.0.tar.gz"
            ),
            Some("https://github.com/BurntSushi/ripgrep.git".into())
        );
    }

    #[test]
    fn parses_aur_rpc() {
        let j =
            r#"{"resultcount":1,"results":[{"Name":"yay","PackageBase":"yay","Version":"1-1"}]}"#;
        let p: AurRpcResponse = serde_json::from_str(j).unwrap();
        assert_eq!(p.results[0].package_base, "yay");
    }
}
