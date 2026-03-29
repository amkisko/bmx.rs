//! Search for installable sources without cloning: GitHub/GitLab APIs plus optional
//! root-directory checks (same markers as `detect_strategy`), AUR RPC, and Homebrew
//! formula metadata (`formulae.brew.sh`).

mod consts;
mod github;
mod gitlab;
mod helpers;
mod http;
mod registries;

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use url::Url;

use crate::cli::SearchBackendArg;
use crate::config::load_config;
use crate::source::normalize_source_base;

use github::search_github;
use gitlab::search_gitlab;
use helpers::{config_with_github_base, config_with_gitlab_base, gitlab_api_v4_root};
use registries::{search_aur, search_homebrew};

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

#[cfg(test)]
mod tests;
