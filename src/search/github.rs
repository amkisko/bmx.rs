use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use url::Url;

use crate::source::normalize_explicit_url;
use crate::types::Config;

use super::consts::ROOT_BUILD_MARKERS;
use super::helpers::github_qualifiers;
use super::hit::SearchHit;
use super::http::{clip, github_headers, http_get_json, http_get_json_soft};
use super::registries::install_hint_resolves;

pub(crate) fn search_github(
    cfg: &Config,
    api_root: &str,
    query: &str,
    limit: usize,
    include_forks: bool,
    probe: bool,
) -> Result<Vec<SearchHit>> {
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

    let mut hits = Vec::new();
    for item in parsed.items {
        if hits.len() >= limit {
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
        let clone_url = item.clone_url.unwrap_or_else(|| {
            normalize_explicit_url(&format!("https://github.com/{owner_repo}.git"))
        });
        hits.push(SearchHit {
            name: owner_repo,
            url: clone_url,
        });
    }
    Ok(hits)
}

pub(crate) fn github_repo_root_buildable(
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

pub(crate) fn github_contents_has_marker(body: &str) -> bool {
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
pub(crate) struct GitHubSearchResponse {
    pub(crate) items: Vec<GitHubRepo>,
}

#[derive(Deserialize)]
pub(crate) struct GitHubRepo {
    pub(crate) full_name: String,
    pub(crate) clone_url: Option<String>,
    pub(crate) archived: bool,
    pub(crate) fork: bool,
    #[serde(default)]
    pub(crate) disabled: bool,
    #[serde(default)]
    pub(crate) default_branch: Option<String>,
}
