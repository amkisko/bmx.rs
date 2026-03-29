use anyhow::{Context, Result};
use serde::Deserialize;
use url::Url;

use crate::source::normalize_explicit_url;
use crate::types::Config;

use super::consts::ROOT_BUILD_MARKERS;
use super::http::{clip, gitlab_headers, http_get_json, http_get_json_soft};
use super::registries::install_hint_resolves;

pub(crate) fn search_gitlab(
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

pub(crate) fn gitlab_repo_root_buildable(
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
pub(crate) struct GitLabTreeEntry {
    pub(crate) name: String,
}

#[derive(Deserialize)]
pub(crate) struct GitLabProject {
    pub(crate) id: u64,
    pub(crate) path_with_namespace: String,
    pub(crate) http_url_to_repo: Option<String>,
    pub(crate) ssh_url_to_repo: Option<String>,
    #[serde(default)]
    pub(crate) empty_repo: Option<bool>,
    #[serde(default)]
    pub(crate) fork: Option<bool>,
    #[serde(default)]
    pub(crate) default_branch: Option<String>,
}
